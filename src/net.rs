/*
 * Herein lies a networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2018,  The Conwayste Developers
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate futures;
extern crate tokio_core;
extern crate bincode;
extern crate semver;

use std::cmp::{PartialEq, PartialOrd, Ordering};
use std::fmt::Debug;
use std::{io, fmt, str, result, time::{Duration, Instant}};
use std::net::{self, SocketAddr};
use std::collections::VecDeque;

use self::tokio_core::net::{UdpSocket, UdpCodec};
use self::tokio_core::reactor::Handle;
use self::futures::sync::mpsc;
use self::bincode::{serialize, deserialize, Infinite};
use self::semver::{Version, SemVerError};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 12345;
const TIMEOUT_IN_SECONDS:    u64   = 5;
const NETWORK_QUEUE_LENGTH: usize = 15;
const RETRANSMISSION_THRESHOLD: Duration = Duration::from_millis(100);

// For unit testing, I cover duplicate sequence numbers. The search returns Ok(index) on a slice with a matching value.
// Instead of returning that index, I return this much larger value and avoid insertion into the queues.
// (110 is the avg weight of an amino acid in daltons :] Much larger than our current queue size)
const MATCH_FOUND_SENTINEL: usize = 110;

//////////////// Public Macros /////////////////

#[macro_export]
macro_rules! netwayste_send {
    // Client
    ($_self:ident, $tx:expr, $dest:expr, ($failmsg:expr $(, $args:expr)*)) => {
        let result = $tx.unbounded_send($dest);
        if result.is_err() {
            warn!($failmsg, $( $args)*);
            $_self.network.statistics.tx_packets_failed += 1;
        } else {
            $_self.network.statistics.tx_packets_success += 1;
        }
    };
    // Server
    ($tx:ident, $dest:expr, ($failmsg:expr $(, $args:expr)*)) => {
        let result = $tx.unbounded_send($dest);
        if result.is_err() {
            warn!($failmsg, $( $args)*);
        }
    };
/*    ($_self:ident, $tx:expr, $dest:expr) => {
        let result = $tx.unbounded_send($dest);
        if result.is_err() {
            error!();
            $_self.network.statistics.tx_keep_alive_failed += 1;
        } else {
            $_self.network.statistics.tx_keep_alive_success += 1;
        }
    };
    */
    // Temp placeholder for client for exit()
    ($tx:expr, ()) => {
        let result = $tx.unbounded_send(());
        if result.is_err() {
            error!("Something really bad is going on... client cannot exit!");
        }
    };
}

//////////////// Error handling ////////////////
#[derive(Debug)]
pub enum NetError {
    AddrParseError(net::AddrParseError),
    IoError(io::Error),
}

impl From<net::AddrParseError> for NetError {
    fn from(e: net::AddrParseError) -> Self {
        NetError::AddrParseError(e)
    }
}

impl From<io::Error> for NetError {
    fn from(e: io::Error) -> Self {
        NetError::IoError(e)
    }
}


////////////////////// Data model ////////////////////////
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum RequestAction {
    None,   // never actually sent
    Connect{name: String, client_version: String},      // server replies w/ OK/PleaseUpgrade; TODO: password
    Disconnect,
    KeepAlive,
    ListPlayers,
    ChatMessage(String),
    ListRooms,
    NewRoom(String),
    JoinRoom(String),
    LeaveRoom,
    TestSequenceNumber(u64),
}

// server response codes -- mostly inspired by https://en.wikipedia.org/wiki/List_of_HTTP_status_codes
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ResponseCode {
    // success - these are all 200 in HTTP
    OK,                              // 200 no data
    LoggedIn(String, String),                // player is logged in -- provide cookie
    PlayerList(Vec<String>),
    RoomList(Vec<(String, u64, bool)>), // (room name, # players, started?)
    // errors
    BadRequest(Option<String>),      // 400 unspecified error that is client's fault
    Unauthorized(Option<String>),    // 401 not logged in
    TooManyRequests(Option<String>), // 429
    ServerError(Option<String>),     // 500
    NotConnected(Option<String>),    // no equivalent in HTTP due to handling at lower (TCP) level
    PleaseUpgrade(Option<String>),   // client should give upgrade msg to user, but continue as if OK
    KeepAlive,                       // Server's heart is beating
    OldPacket,                       // Internally used to ignore a packet, just for testing
}

// chat messages sent from server to all clients other than originating client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastChatMessage {
    pub chat_seq:    Option<u64>,   // Some(<number>) when sent to clients (starts at 0 for first
                                    // chat message sent to this client in this room); None when
                                    // internal to server
    pub player_name: String,
    pub message:     String,        // should not contain newlines
}

impl PartialEq for BroadcastChatMessage {
    fn eq(&self, other: &BroadcastChatMessage) -> bool {
        let self_seq_num = self.sequence_number();
        let other_seq_num = other.sequence_number();
        self_seq_num == other_seq_num
    }
}

impl Eq for BroadcastChatMessage {}

impl PartialOrd for BroadcastChatMessage {
    fn partial_cmp(&self, other: &BroadcastChatMessage) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BroadcastChatMessage {
    fn cmp(&self, other: &BroadcastChatMessage) -> Ordering {
        let self_seq_num = self.sequence_number();
        let other_seq_num = other.sequence_number();

        self_seq_num.cmp(&other_seq_num)
    }
}

impl BroadcastChatMessage {
    pub fn new(sequence: u64, name: String, msg: String) -> BroadcastChatMessage {
        BroadcastChatMessage {
            chat_seq: Some(sequence),
            player_name: name,
            message: msg
        }
    }

    fn sequence_number(&self) -> u64 {
        if let Some(v) = self.chat_seq {
            v
        } else { 0 }
    }
}

// TODO: adapt or import following from libconway
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GenState {
    // state of the Universe
    pub gen:        u64,
    pub dummy_data: u8,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GenDiff  {
    // difference between states of Universe
    pub old_gen:    u64,
    pub new_gen:    u64,
    pub dummy_data: u8,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GameOutcome {
    pub winner: Option<String>,     // Some(<name>) if winner, or None, meaning it was a tie/forfeit
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum GameUpdateType {
    GameStart,
    NewUserList(Vec<String>),   // list of names of all users including current user
    GameFinish(GameOutcome),
    GameClose,   // kicks user back to arena
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GameUpdate {
    pub game_update_seq: Option<u64>,  // see BroadcastChatMessage chat_seq field for Some/None meaning
    update_type:         GameUpdateType,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum UniUpdateType {
    State(GenState),
    Diff(GenDiff),
    NoChange,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Packet {
    Request {
        // sent by client
        sequence:        u64,
        response_ack:    Option<u64>,    // most recent response sequence number received
        cookie:          Option<String>, // present if and only if action != connect
        action:          RequestAction,
    },
    Response {
        // sent by server in reply to client
        sequence:        u64,
        request_ack:     Option<u64>,     // most recent request sequence number received
        code:            ResponseCode,
    },
    Update {
        // in-game: sent by server
        chats:           Option<Vec<BroadcastChatMessage>>, // All non-acknowledged chats are sent each update
        game_updates:    Option<Vec<GameUpdate>>,           //
        universe_update: UniUpdateType,                     //
    },
    UpdateReply {
        // in-game: sent by client in reply to server
        cookie:               String,
        last_chat_seq:        Option<u64>, // sequence number of latest chat msg. received from server
        last_game_update_seq: Option<u64>, // seq. number of latest game update from server
        last_gen:             Option<u64>, // generation number client is currently at
    }
}

impl Packet {
    pub fn sequence_number(&self) -> u64 {
        if let Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } = self {
            *sequence
        } else if let Packet::Response{ sequence, request_ack: _, code: _ } = self {
            *sequence
        } else if let Packet::Update{ chats: _, game_updates: _, universe_update } = self {
            match universe_update {
                UniUpdateType::State(gs) => { gs.gen },
                UniUpdateType::Diff(gd) => { gd.new_gen },
                UniUpdateType::NoChange => 0
            }
        } else {
            unimplemented!(); // UpdateReply is not saved
        }
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Packet::Request{ sequence, response_ack, cookie, action } => {
                write!(f, "[Request] cookie: {:?} sequence: {} ack: {:?} event: {:?}", cookie, sequence, response_ack, action)
            }
            Packet::Response{ sequence, request_ack, code } => {
                write!(f, "[Response] sequence: {} ack: {:?} event: {:?}", sequence, request_ack, code)
            }
            Packet::Update{ chats: _, game_updates, universe_update } => {
                write!(f, "[Update] game_updates: {:?} universe_update: {:?}", game_updates, universe_update)
            }
            #[cfg(not(test))]
            _ => {unimplemented!()}
            #[cfg(test)]
            _ => {Result::Ok(())}
        }
    }
}

impl PartialEq for Packet {
    fn eq(&self, other: &Packet) -> bool {
        let self_seq_num = self.sequence_number();
        let other_seq_num = other.sequence_number();
        self_seq_num == other_seq_num
    }
}

impl Eq for Packet {}

impl PartialOrd for Packet {
    fn partial_cmp(&self, other: &Packet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Packet {
    fn cmp(&self, other: &Packet) -> Ordering {
        let self_seq_num = self.sequence_number();
        let other_seq_num = other.sequence_number();

        self_seq_num.cmp(&other_seq_num)
    }
}

//////////////// Packet (de)serialization ////////////////
pub struct LineCodec;
impl UdpCodec for LineCodec {
    type In = (SocketAddr, Option<Packet>);   // if 2nd element is None, it means deserialization failure
    type Out = (SocketAddr, Packet);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        match deserialize(buf) {
            Ok(decoded) => Ok((*addr, Some(decoded))),
            Err(e) => {
                let local: SocketAddr = format!("{}:{}", "127.0.0.1", DEFAULT_PORT.to_string()).parse().unwrap();
                // We only want to warn when the incoming packet is external to the host system
                if local != *addr {
                    warn!("WARNING: error during packet deserialization: {:?}", e);
                }
                Ok((*addr, None))
            }
        }
    }

    fn encode(&mut self, (addr, player_packet): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        let encoded: Vec<u8> = serialize(&player_packet, Infinite).unwrap();
        into.extend(encoded);
        addr
    }
}

//////////////// Network interface ////////////////
#[allow(dead_code)]
pub fn bind(handle: &Handle, opt_host: Option<&str>, opt_port: Option<u16>) -> Result<UdpSocket, NetError> {

    let host = if let Some(host) = opt_host { host } else { DEFAULT_HOST };
    let port = if let Some(port) = opt_port { port } else { DEFAULT_PORT };
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let sock = UdpSocket::bind(&addr, &handle).expect("failed to bind socket");
    Ok(sock)
}

#[allow(dead_code)]
pub fn get_version() -> result::Result<Version, SemVerError> {
    Version::parse(VERSION)
}

pub fn has_connection_timed_out(heartbeat: Option<Instant>) -> bool {
    if let Some(heartbeat) = heartbeat {
        (Instant::now() - heartbeat) > Duration::from_secs(TIMEOUT_IN_SECONDS)
    } else { false }
}

pub struct NetworkStatistics {
    pub tx_packets_failed: u64,
    pub tx_packets_success: u64,
}

impl NetworkStatistics {
    fn new() -> Self {
        NetworkStatistics {
            tx_packets_failed: 0,
            tx_packets_success: 0,
        }
    }

    fn reset(&mut self) {
        #![deny(unused_variables)]
        let Self {
            ref mut tx_packets_failed,
            ref mut tx_packets_success,
        } = *self;
        *tx_packets_failed     = 0;
        *tx_packets_success    = 0;
    }
}


pub trait Sequenced: Ord {
    fn sequence_number(&self) -> u64;
}

impl Sequenced for Packet {
    fn sequence_number(&self) -> u64 {
        self.sequence_number()
    }
}

impl Sequenced for BroadcastChatMessage {
    fn sequence_number(&self) -> u64 {
        self.sequence_number()
    }
}

pub trait NetworkQueue<T: Ord+Sequenced+Debug> {
    fn new(size: usize) -> ItemQueue<T>
    {
        ItemQueue::<T>::with_capacity(size)
    }

    fn head_of_queue(&self) -> Option<&T> {
        self.as_queue_type().back()
    }

    fn tail_of_queue(&self) -> Option<&T> {
        self.as_queue_type().front()
    }

    fn newest_seq_num(&self) -> Option<u64> {
        let opt_newest_packet: Option<&T> = self.head_of_queue();

        if opt_newest_packet.is_some() {
            let newest_packet: &T = opt_newest_packet.unwrap();
            Some(newest_packet.sequence_number())
        } else { None }
    }

    fn oldest_seq_num(&self) -> Option<u64> {
        let opt_oldest_packet: Option<&T> = self.tail_of_queue();

        if opt_oldest_packet.is_some() {
            let oldest_packet: &T = opt_oldest_packet.unwrap();
            Some(oldest_packet.sequence_number())
        } else { None }
    }

    fn push_back(&mut self, item: T) {
        self.as_queue_type_mut().push_back(item);
    }

    fn push_front(&mut self, item: T) {
        self.as_queue_type_mut().push_front(item);
    }

    fn len(&self) -> usize {
        self.as_queue_type().len()
    }

    fn insert(&mut self, index: usize, item: T) {
        self.as_queue_type_mut().insert(index, item);
    }

    /// I've deemed 'far away' to mean the half of the max value of the type.
    fn is_seq_sufficiently_far_away(&self, a: u64, b: u64) -> bool {
        static HALFWAYPOINT: u64 = u64::max_value()/2;
        if a > b {
            a - b > HALFWAYPOINT
        } else {
            b - a > HALFWAYPOINT
        }
    }

    /// Checks if the insertion of `sequence` induces a newly wrapped queue state.
    /// If we have already wrapped in the buffer, then it shouldn't cause another wrap due to the nature of the problem.
    fn will_seq_cause_a_wrap(&self,
                            buffer_wrap_index: Option<usize>,
                            sequence: u64,
                            oldest_seq_num: u64,
                            newest_seq_num: u64) -> bool {
        if buffer_wrap_index.is_none() {
            self.is_seq_sufficiently_far_away(sequence, oldest_seq_num)
            && self.is_seq_sufficiently_far_away(sequence, newest_seq_num)
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.as_queue_type_mut().clear();
    }

    fn remove(&mut self, pkt: &T) -> T {
        let result = {
            let search_space: Vec<&T> = self.as_queue_type_mut().iter().collect();
            search_space.as_slice().binary_search(&pkt)
        };
        match result {
            Err(_) => panic!("Could not remove transmitted item from queue"),
            Ok(index) => {
                let pkt = self.as_queue_type_mut().remove(index).unwrap();
                return pkt;
            }
        }
    }

    fn pop_front_with_count(&mut self, mut num_to_remove: usize) {
        while num_to_remove > 0 {
            self.as_queue_type_mut().pop_front();
            num_to_remove -= 1;
        }
    }

    fn discard_older_items(&mut self);
    fn buffer_item(&mut self, item: T) -> bool;
    fn as_queue_type(&self) -> &ItemQueue<T>;
    fn as_queue_type_mut(&mut self) -> &mut ItemQueue<T>;
}

type ItemQueue<T> = VecDeque<T>;

#[derive(Debug)]
pub struct TXQueue {
    pub queue: ItemQueue<Packet>,
    pub timestamps: VecDeque<Instant>,
}

pub struct RXQueue<T> {
    pub queue: ItemQueue<T>,
    pub buffer_wrap_index: Option<usize>,
}

impl NetworkQueue<Packet> for TXQueue {
    fn as_queue_type(&self) -> &ItemQueue<Packet> {
        &self.queue
    }

    fn as_queue_type_mut(&mut self) -> &mut ItemQueue<Packet> {
        &mut self.queue
    }

    /// This will keep the specified queue under the NETWORK_QUEUE_LENGTH limit.
    /// The TX queue needs to ensure a spot is open if we're at capacity.
    fn discard_older_items(&mut self) {
        let queue = self.as_queue_type_mut();
        let queue_size = queue.len();
        if queue_size >= NETWORK_QUEUE_LENGTH {
            for _ in 0..(queue_size-NETWORK_QUEUE_LENGTH) {
                queue.pop_front();
            }
            queue.pop_front(); // Always keep one empty for TX queues
        }
    }

    /// As we buffer new packets, we'll want to throw away the older packets.
    /// We must be careful to ensure that we do not throw away packets that have
    /// not yet been acknowledged by the end-point.
    fn buffer_item(&mut self, item: Packet) -> bool {
        let sequence = match item {
            Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } => sequence,
            Packet::Response{ sequence, request_ack: _, code: _ } => sequence,
            _ => return false, //do nothing
        };

        let opt_head_seq_num: Option<u64> = self.newest_seq_num();

        if opt_head_seq_num.is_none() {
            self.push_back(item);
            self.timestamps.push_back(Instant::now());
            return false;
        }

        let head_seq_num = opt_head_seq_num.unwrap(); // unwrap safe

        // ameen: re-evaluate need
        //self.discard_older_items();

        // Current item is newer, and we're adding in sequential order
        if head_seq_num < sequence && head_seq_num+1 == sequence {
            self.push_back(item);
            self.timestamps.push_back(Instant::now());
        }

        false   // Will need to scan if already present in packet
    }

    fn remove(&mut self, pkt: &Packet) -> Packet {
        // In the TX queue case, removing a Packet::Response means its request_ack is equivalent to
        // the Packet::Request's sequence number which  actually resides in the queue
        let opt_search_id: &Option<u64> = match pkt {
            // TODO double check Packet::Request when implementing Server-side
            Packet::Request{ sequence: _, response_ack, cookie: _, action: _ } => response_ack,
            Packet::Response{ sequence: _, request_ack, code: _ } => request_ack,
            _ => unimplemented!() // !("Update/UpdateReply not yet implemented")
        };
        let search_id = if let &Some(si) = opt_search_id { si } else { panic!("Cannot remove because the 'ack' was None") };
        let dummy_packet = Packet::Request{
            sequence: search_id,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        let result = {
            let search_space: Vec<&Packet> = self.as_queue_type_mut().iter().collect();
            search_space.as_slice().binary_search(&&dummy_packet)
        };
        match result {
            Err(_) => panic!("Could not remove transmitted item from TX queue. Trying to remove {:?}, from {:?}",
                             dummy_packet, self),
            Ok(index) => {
                let pkt = self.as_queue_type_mut().remove(index).unwrap();
                self.timestamps.remove(index);
                return pkt;
            }
        }
    }

    fn clear(&mut self) {
        let Self {
            ref mut queue,
            ref mut timestamps,
        } = *self;

        queue.clear();
        timestamps.clear();
    }
}

impl TXQueue {
    pub fn get_retransmit_indices(&self) -> Vec<usize> {
        let iter = self.timestamps.iter();
        iter.enumerate()
            .filter(|(_,&ts)| (Instant::now() - ts) >= RETRANSMISSION_THRESHOLD)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>()
    }
}

impl<T> NetworkQueue<T> for RXQueue<T>
        where T: Sequenced+Debug {

    fn as_queue_type(&self) -> &ItemQueue<T> {
        &self.queue
    }

    fn as_queue_type_mut(&mut self) -> &mut ItemQueue<T> {
        &mut self.queue
    }

    fn discard_older_items(&mut self) {
        let queue = self.as_queue_type_mut();
        let queue_size = queue.len();
        if queue_size >= NETWORK_QUEUE_LENGTH {
            for _ in 0..(queue_size-NETWORK_QUEUE_LENGTH) {
                queue.pop_front();
            }
        }
    }

    /// Upon packet receipt, we must maintain linearly increasing sequence number order of received items of type `T`.
    /// In a perfect world, all `T`'s arrive in order, but this is not the case in reality. All T's are `Sequenced and
    /// have a corresponding sequence number to delineate order.
    ///
    /// This also handles the case where the sequence number numerically wraps.
    /// `buffer_wrap_index` is maintained to denote the queue index at which the numerical wrap occurs.
    /// It is used to determine if a wrap has occurred yet, and if so, helps narrow the queue subset we will insert into.
    ///
    /// For sequence numbers of type 'Byte':
    ///     [253, 254, 1, 2, 4]
    ///                ^ wrapping index
    ///     Inserting '255' performs a search on the subset [253, 254] only.
    ///     After insertion, the wrapping index increments since we modified the left half.
    ///     [253, 254, 255, 1, 2, 4]
    ///                     ^ wrapping index
    ///     Inserting '3' performs a linear search on [1, 2, 4]. Does not impact wrapping index.
    ///     [253, 254, 255, 1, 2, 3, 4]
    ///                     ^ wrapping index
    ///
    /// Because we can receive `T`'s out-of-order even when wrapped, there are checks added below to safeguard
    /// against this. Primarily, they cover the cases where out-of-order insertion would transition the queue into a
    /// wrapped state from a non-wrapped state.
    fn buffer_item(&mut self, item: T) -> bool {
        let mut packet_exists: bool = false;
        let sequence = item.sequence_number();

        // Empty queue
        let opt_head_seq_num: Option<u64> = self.newest_seq_num();
        if opt_head_seq_num.is_none() {
            self.push_back(item);
            return packet_exists;
        }
        let opt_tail_seq_num: Option<u64> = self.oldest_seq_num();
        let newest_seq_num = opt_head_seq_num.unwrap();
        let oldest_seq_num = opt_tail_seq_num.unwrap();

        if sequence < oldest_seq_num {
            // Special case with max_value where we do not need to search for the insertion spot.
            if newest_seq_num == u64::max_value() {
                if self.will_seq_cause_a_wrap(self.buffer_wrap_index, sequence, oldest_seq_num, newest_seq_num) {
                    self.push_back(item);
                    self.buffer_wrap_index = Some(self.len() - 1);
                } else {
                    self.push_front(item);
                }
            } else if sequence > newest_seq_num && self.buffer_wrap_index.is_some() {
                // When wrapped, either this is the newest sequence number so far, or
                // an older sequence number arrived late.
                if self.is_seq_sufficiently_far_away(sequence, newest_seq_num) {
                    if let Some(buffer_wrap_index) = self.buffer_wrap_index {
                        let insertion_index = self.find_rx_insertion_index_in_subset(0, buffer_wrap_index, &item);
                        self.buffer_wrap_index = Some(buffer_wrap_index + 1);
                        packet_exists = self.insert_into_rx_queue(insertion_index, item);
                    }
                } else {
                    self.push_back(item);
                }
            } else if sequence < newest_seq_num {
                // The new seq num appears to be older than everything,
                // but it may be far enough in value to induce a wrap.
                let insertion_index: Option<usize>;
                if self.will_seq_cause_a_wrap(self.buffer_wrap_index, sequence, oldest_seq_num, newest_seq_num) {
                    insertion_index = Some(self.len());
                    self.buffer_wrap_index = insertion_index;
                } else if let Some(buffer_wrap_index) = self.buffer_wrap_index {
                    insertion_index = self.find_rx_insertion_index_in_subset(buffer_wrap_index, self.len(), &item);
                } else {
                    insertion_index = self.find_rx_insertion_index(&item);
                }

                packet_exists = self.insert_into_rx_queue(insertion_index, item);
            } else {
                // Smallest sequence number (in value) that we have seen thus far.
                self.push_front(item);

                if self.buffer_wrap_index.is_some() {
                    self.buffer_wrap_index = Some(self.buffer_wrap_index.unwrap() + 1);
                }
                panic!("Previously thought to be dead code. Prove us wrong!");
            }
        } else { // Sequence >= oldest_seq_num
            let insertion_index: Option<usize>;
            if sequence < newest_seq_num {
                insertion_index = self.find_rx_insertion_index(&item);
            } else {
                // Greater than the oldest and newest seq num in the queue.
                // Time to see if we have wrapped already, and if not, we
                // need to see if we are about to wrap based on this insertion.
                if let Some(buffer_wrap_index) = self.buffer_wrap_index {
                    insertion_index = self.find_rx_insertion_index_in_subset(0, buffer_wrap_index, &item);
                    self.buffer_wrap_index = Some(buffer_wrap_index + 1);
                } else {
                    if self.will_seq_cause_a_wrap(self.buffer_wrap_index, sequence, oldest_seq_num, newest_seq_num) {
                        // Sequence is far enough, and we haven't wrapped, so it arrived late.
                        // Push it to the front of the queue
                        insertion_index = Some(0);
                        self.buffer_wrap_index = Some(1);
                    } else {
                        // No wrap yet, and not about to either, use a blind binary search.
                        insertion_index = self.find_rx_insertion_index(&item);
                    }
                }
            }
            packet_exists = self.insert_into_rx_queue(insertion_index, item);
        }
        packet_exists
    }
}

impl<T> RXQueue<T> where T: Sequenced+Debug {

    /// Search within the RX queue, but we have no idea where to insert.
    /// This should cover only within the RX queue and not at the edges (front or back).
    /// We accomplish this by splitting the VecDequeue into a slice tuple and then binary searching on each slice.
    /// Small note: The splitting of VecDequeue is into its 'front' and 'back' halves, based on how 'push_front'
    /// and 'push_back' were used.
    fn find_rx_insertion_index(&self, item: &T) -> Option<usize> {
        let (front_slice, back_slice) = self.queue.as_slices();
        let f_result = front_slice.binary_search(&item);
        let b_result = back_slice.binary_search(&item);

        match (f_result, b_result) {
            (Err(loc1), Err(loc2)) => {
                // We will not insert at the front of front_slice or end of back_slice,
                // because these cases are covered by "oldest" and "newest" already.
                // This leaves us with:
                //      1. a) At the end of the front slice
                //         b) Somewhere in the middle of front slice
                //      2. a) At the start of the back slice (same as "end of front slice")
                //         b) Somewhere in the middle of the back slice
                // loc1 and loc2 are the index at which we would insert at each slice
                match (loc1, loc2) {
                    // Case 1a)/2a)
                    (g, 0) if g == front_slice.len() => Some(front_slice.len()),
                    // Case 2b)
                    (g, n) if g == front_slice.len() => Some(front_slice.len() + n),
                    // Case 1b)
                    (n, 0) => Some(n),
                    // Could not find a place to insert
                    (_, _) => None,
                }
            },
            #[cfg(test)]
            (_,_) => Some(MATCH_FOUND_SENTINEL),
            #[cfg(not(test))]
            (_,_) => None,
        }
    }

    // Search within the RX queue when we know which subset interests us.
    fn find_rx_insertion_index_in_subset(&self, start: usize, end: usize, item: &T) -> Option<usize> {
        let search_space: Vec<&T> = self.queue.iter().skip(start).take(end).collect();
        let result = search_space.as_slice().binary_search(&item);
        match result {
            Err(loc) => Some(loc + start),
            #[cfg(test)]
            Ok(_) => Some(MATCH_FOUND_SENTINEL),
            #[cfg(not(test))]
            Ok(_) => None,
        }
    }

    // Checked insertion against the sentinel used during unit testing
    fn insert_into_rx_queue(&mut self, index: Option<usize>, item: T) -> bool {
        let mut exists: bool = false;
        if let Some(insertion_index) = index {
            if insertion_index != MATCH_FOUND_SENTINEL {
                #[cfg(test)]
                self.as_queue_type_mut().insert(insertion_index, item);
            }
            #[cfg(not(test))]
            self.as_queue_type_mut().insert(insertion_index, item);
        } else { exists = true; } // Packet is present in queue, hence None.
        return exists;
    }

    // The requirement is that what we return must implement Iterator, and TakeWhile fulfill this. Pretty neat.
    // Seq_num as a parameter specifies the starting sequence number to iterate over. Since packets can arrive
    // out-of-order, the rx queue may be contiguous but not complete.
    // Ex: Assume packet SN we're waiting for an ack is 10, but the rx queue contains [12, 13, 14, 16]
   // pub fn get_contiguous_packets_count(&self, mut seq_num: u64) -> impl Iterator<Item=&T> {
    pub fn get_contiguous_packets_count(&self, mut seq_num: u64) -> usize {
        let iter = self.queue.iter().take_while(move |x| {
            let ready = x.sequence_number() == seq_num;
            if ready {
                seq_num += 1;
            }
            ready
        });
        iter.count()
    }
}

pub struct NetworkManager {
    pub statistics:       NetworkStatistics,
    pub tx_packets:       TXQueue,                               // Back = Newest, Front = Oldest
    pub rx_packets:       RXQueue<Packet>,                       // Back = Newest, Front = Oldest
    pub rx_chat_messages: Option<RXQueue<BroadcastChatMessage>>, // Back = Newest, Front = Oldest;
                                                                 //     Messages are drained into the Client;
                                                                 //     Server does not use this structure.
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            statistics: NetworkStatistics::new(),
            tx_packets:  TXQueue{ queue: TXQueue::new(NETWORK_QUEUE_LENGTH), timestamps: VecDeque::<Instant>::new() },
            rx_packets:  RXQueue{ queue: RXQueue::<Packet>::new(NETWORK_QUEUE_LENGTH),
                                                                buffer_wrap_index: None },
            rx_chat_messages: None,
        }
    }

    pub fn with_message_buffering(self) -> NetworkManager {
        NetworkManager {
            statistics: self.statistics,
            tx_packets: self.tx_packets,
            rx_packets: self.rx_packets,
            rx_chat_messages:  Some(RXQueue {
                                        queue: RXQueue::<BroadcastChatMessage>::new(NETWORK_QUEUE_LENGTH),
                                        buffer_wrap_index: None,
                                }),
        }
    }

    pub fn reset(&mut self) {
        #![deny(unused_variables)]
        let Self {
            ref mut statistics,
            ref mut tx_packets,
            ref mut rx_packets,
            ref mut rx_chat_messages,
        } = *self;
        statistics.reset();
        tx_packets.clear();
        rx_packets.clear();
        if let Some(chat_messages) = rx_chat_messages {
            chat_messages.clear();
            chat_messages.buffer_wrap_index = None;
        }
    }

    pub fn print_statistics(&self) {
        info!("Tx Successes: {}", self.statistics.tx_packets_success);
        info!("Tx Failures:  {}", self.statistics.tx_packets_failed);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{thread, time::{Instant, Duration}};

    #[test]
    fn test_discard_older_packets_empty_queue() {
        let mut nm = NetworkManager::new();

        nm.tx_packets.discard_older_items();
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), 0);
        assert_eq!(nm.rx_packets.len(), 0);
    }

    #[test]
    fn test_discard_older_packets_under_limit_keeps_all_messages() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.push_back(pkt.clone());
        nm.tx_packets.push_back(pkt.clone());
        nm.tx_packets.push_back(pkt.clone());

        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), 3);

        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());

        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), 3);
    }

    #[test]
    fn test_discard_older_packets_equal_to_limit() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH);
        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH-1);

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
    }

    #[test]
    fn test_discard_older_packets_exceeds_limit_retains_max() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH+10 {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH+10);
        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH-1);

        for _ in 0..NETWORK_QUEUE_LENGTH+5 {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH+5);
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
    }

    #[test]
    fn test_buffer_item_queue_is_empty() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 1);
    }

    #[test]
    fn test_buffer_item_sequence_number_reused() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = nm.tx_packets.queue.back().unwrap();
        if let Packet::Request { sequence: _, response_ack: _, cookie: _, action } = pkt {
            assert_eq!(*action, RequestAction::None);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequencing() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 2);
    }

    #[test]
    fn test_buffer_item_newer_packet_has_smaller_sequence_number() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 1);

        let pkt = nm.tx_packets.queue.back().unwrap();
        if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
            assert_eq!(*sequence, 1);
        }
    }

/*
    #[test]
    fn test_buffer_item_max_queue_limit_maintained() {
        let mut nm = NetworkManager::new();
        for index in 0..NETWORK_QUEUE_LENGTH+5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.tx_packets.buffer_item(pkt);
        }

        let mut iter =  nm.tx_packets.queue.iter();
        for index in 5..NETWORK_QUEUE_LENGTH+5 {
            let pkt = iter.next().unwrap();
            if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
                assert_eq!(*sequence, index as u64);
            }
        }
    }
*/

    #[test]
    fn test_buffer_item_basic_contiguous_ascending() {
        let mut nm = NetworkManager::new();
        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_contiguous_descending() {
        let mut nm = NetworkManager::new();
        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequential_gap_ascending() {
        let mut nm = NetworkManager::new();
        // TODO Replace with (x,y).step_by(z) once stable
        for index in [0,2,4,6,8,10].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequential_gap_descending() {
        let mut nm = NetworkManager::new();
        for index in [0,2,4,6,8,10].iter().rev() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_basic_random() {
        let mut nm = NetworkManager::new();
        for index in [5, 2, 9, 1, 0, 8, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 5, 6, 8, 9].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_butterfly_pattern() {
        let mut nm = NetworkManager::new();
        // This one is fun because it tests the internal edges of (front_slice and back_slice)
        for index in [0, 10, 1, 9, 2, 8, 3, 7, 4, 6, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_repetition() {
        let mut nm = NetworkManager::new();
        for index in [0, 0, 0, 0, 1, 2, 2, 2, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 5].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_advanced_sequential_then_pseudorandom_then_sequential() {
        let mut nm = NetworkManager::new();

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in 13..20 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_advanced_reverse_sequential_then_random_then_reverse_sequential() {
        let mut nm = NetworkManager::new();

        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in (13..20).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_wrapping_case() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let start = u64_max - 5;

        for index in start..(start+5) {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_basic_wrapping_case_then_out_of_order() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let start = u64_max - 5;

        for index in start..(start+5) {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [5, 0, 4, 1, 3, 2].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4, 5].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_everything_out_of_order() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [ max_minus_4,
                            two,
                            max_minus_1,
                            max_minus_5,
                            u64_max,
                            three,
                            max_minus_2,
                            zero,
                            max_minus_3,
                            one ];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned()); // Add in u64 max value plus others

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_max_sequence_number_arrives_after_a_wrap() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_1, max_minus_2, three, u64_max, two];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_2, max_minus_1, u64_max, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_oldest_sequence_number_arrived_last() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_1, max_minus_2, three, u64_max, two, one, zero, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrap_occurs_with_two_item_queue() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        // Forward wrap occurs non-contiguously (aka [254, 0, ...] for bytes)
        let input_order = [max_minus_1, zero, three, u64_max, max_minus_2, one, two, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrap_occurs_with_two_item_queue_in_reverse() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        // Wrap takes place in reverse order ( aka [0, 254, ...] for bytes)
        let input_order = [zero, max_minus_1, three, u64_max, max_minus_2, one, two, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_max_arrives_first() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [u64_max, max_minus_4, two, max_minus_1, max_minus_5, three, max_minus_2, zero, max_minus_3, one];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_sequence_number_descending() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [three, two, one, zero, u64_max, max_minus_1, max_minus_2, max_minus_3, max_minus_4, max_minus_5];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_sequence_number_alternating() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_5, three, max_minus_4, two, max_minus_3, one, max_minus_2, zero, max_minus_1, u64_max];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned()); // Add in u64 max value plus others

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_reinitialize_all_queues_cleared() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH);

        let chat_msg = BroadcastChatMessage::new(0, "chatchat".to_owned(), "chatchat".to_owned());
    }

    #[test]
    fn test_get_contiguous_packets_iter() {
        let mut nm = NetworkManager::new();
        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }
        for index in 8..10 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let count = nm.rx_packets.get_contiguous_packets_count(0);
        assert_eq!(count, 5);
        let mut iter = nm.rx_packets.as_queue_type().iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
            // Verify that the packet is not dequeued
            assert_eq!(index, nm.rx_packets.as_queue_type().get(index).unwrap().sequence_number() as usize);
        }
    }

    #[test]
    fn test_get_retransmit_indices() {
        let mut nm = NetworkManager::new();
        for i in 0..5 {
            let pkt = Packet::Request {
                sequence: i,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };

            nm.tx_packets.buffer_item(pkt.clone());

            if i < 3 {
                let timestamp = nm.tx_packets.timestamps.back_mut().unwrap();
                *timestamp = Instant::now() - Duration::from_secs(i+1)
            }
        }
        assert_eq!(nm.tx_packets.get_retransmit_indices().len(), 3);
        thread::sleep(Duration::from_millis(200));
        assert_eq!(nm.tx_packets.get_retransmit_indices().len(), 5);
    }
}
