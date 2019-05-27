/*
 * Herein lies a networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2018-2019 The Conwayste Developers
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

use std::cmp::{PartialEq, PartialOrd, Ordering};
use std::fmt::Debug;
use std::{io, fmt, str, result, time::{Duration, Instant}};
use std::net::{self, SocketAddr};
use std::collections::VecDeque;

use futures::sync::mpsc;
use tokio_core::net::{UdpSocket, UdpCodec};
use tokio_core::reactor::Handle;
use bincode::{serialize, deserialize, Infinite};
use semver::{Version, SemVerError};
use serde::{Serialize, Deserialize};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 2016;
pub const TIMEOUT_IN_SECONDS:    u64   = 5;
pub const NETWORK_QUEUE_LENGTH: usize = 600;        // spot testing with poor network (~675 cmds) showed a max of ~512 length
                                                // keep this for now until the performance issues are resolved
const RETRANSMISSION_THRESHOLD_IN_MS: Duration = Duration::from_millis(400);
const RETRY_THRESHOLD_IN_MS: usize = 2;             //
const RETRY_AGGRESSIVE_THRESHOLD_IN_MS: usize = 5;
const RETRANSMISSION_COUNT: usize = 32;   // Testing some ideas out:. Resend length 16x2, 16=libconway::history_size)

// For unit testing, I cover duplicate sequence numbers. The search returns Ok(index) on a slice with a matching value.
// Instead of returning that index, I return this much larger value and avoid insertion into the queues.
// (110 is the avg weight of an amino acid in daltons :] Much larger than our current queue size)
const MATCH_FOUND_SENTINEL: usize = 110;

//////////////// Public Macros /////////////////

#[macro_export]
macro_rules! netwayste_send {
    ($tx:ident, $dest:expr, ($failmsg:expr $(, $args:expr)*)) => {
        let result = $tx.unbounded_send($dest);
        if result.is_err() {
            warn!($failmsg, $( $args)*);
        }
    };
    // for client for exit()
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
    Connect{name: String, client_version: String},
    Disconnect,
    KeepAlive(u64),     // Send latest response ack on each heartbeat
    ListPlayers,
    ChatMessage(String),
    ListRooms,
    NewRoom(String),
    JoinRoom(String),
    LeaveRoom,
}

// server response codes -- mostly inspired by https://en.wikipedia.org/wiki/List_of_HTTP_status_codes
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ResponseCode {
    // success - these are all 200 in HTTP
    OK,                              // 200 no data
    LoggedIn(String, String),        // player is logged in -- (cookie, server version)
    JoinedRoom(String),              // player has joined the room
    LeaveRoom,                       // player has left the room
    PlayerList(Vec<String>),         // list of players in room or lobby
    RoomList(Vec<(String, u64, bool)>), // (room name, # players, game has started?

    // errors
    BadRequest(Option<String>),      // 400 unspecified error that is client's fault
    Unauthorized(Option<String>),    // 401 not logged in
    TooManyRequests(Option<String>), // 429
    ServerError(Option<String>),     // 500
    NotConnected(Option<String>),    // no equivalent in HTTP due to handling at lower (TCP) level
    KeepAlive,                       // Server's heart is beating
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
    #[allow(unused)]
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
        response_ack:    Option<u64>,    // Next expected  sequence number the Server responds with to the Client.
                                         // Stated differently, the client has seen Server responses from 0 to response_ack-1.
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
    #[allow(unused)]
    pub fn sequence_number(&self) -> u64 {
        if let Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } = self {
            *sequence
        } else if let Packet::Response{ sequence, request_ack: _, code: _ } = self {
            *sequence
        } else if let Packet::Update{ chats: _, game_updates: _, universe_update } = self {
            // TODO revisit once mechanics are fleshed out
            match universe_update {
                UniUpdateType::State(gs) => { gs.gen },
                UniUpdateType::Diff(gd) => { gd.new_gen },
                UniUpdateType::NoChange => 0
            }
        } else {
            unimplemented!(); // UpdateReply is not saved
        }
    }

    #[allow(unused)]
    pub fn set_response_sequence(&mut self, new_ack: Option<u64>) {
        if let Packet::Request{ sequence: _, ref mut response_ack, cookie: _, action: _ } = *self {
            *response_ack = new_ack;
        }
        else if let Packet::Response{ sequence: _, ref mut request_ack, code: _ } = *self {
            *request_ack = new_ack;
        }
        else {
            unimplemented!();
        }
    }

    #[allow(unused)]
    pub fn response_sequence(&self) -> u64 {
        if let Packet::Request{sequence: _, ref response_ack, cookie: _, action: _} = *self {
            if let Some(response_ack) = response_ack
            {
                *response_ack
            } else {
                0
            }
        } else {
            unimplemented!();
        }
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Packet::Request{ sequence, response_ack, cookie, action } => {
                write!(f, "[Request] cookie: {:?} sequence: {} resp_ack: {:?} event: {:?}", cookie, sequence, response_ack, action)
            }
            Packet::Response{ sequence, request_ack, code } => {
                write!(f, "[Response] sequence: {} req_ack: {:?} event: {:?}", sequence, request_ack, code)
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
#[allow(dead_code)]
pub struct LineCodec;
impl UdpCodec for LineCodec {
    type In = (SocketAddr, Option<Packet>);   // if 2nd element is None, it means deserialization failure
    type Out = (SocketAddr, Packet);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        match deserialize(buf) {
            Ok(decoded) => Ok((*addr, Some(decoded))),
            Err(_) => {
                /*
                // TODO: do not create this SocketAddr every time a packet arrives!
                // TODO: DEFAULT_PORT could be wrong. We need to know the real port
                let local: SocketAddr = format!("{}:{}", "127.0.0.1", DEFAULT_PORT.to_string()).parse().unwrap();
                // We only want to warn when the incoming packet is external to the host system
                if local != *addr {
                    warn!("WARNING: error during packet deserialization: {:?}", e);
                }
                */
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

#[allow(dead_code)]
pub fn has_connection_timed_out(heartbeat: Option<Instant>) -> bool {
    if let Some(heartbeat) = heartbeat {
        (Instant::now() - heartbeat) > Duration::from_secs(TIMEOUT_IN_SECONDS)
    } else { false }
}

pub struct NetworkStatistics {
    pub tx_packets_failed: u64,     // From the perspective of the Network OSI layer
    pub tx_packets_success: u64,    // From the perspective of the Network OSI layer
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

pub trait NetworkQueue<T: Ord+Sequenced+Debug+Clone> {
    fn new() -> Self;

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

    fn remove(&mut self, pkt: &T) -> Option<T>;

    fn discard_older_items(&mut self);
    fn buffer_item(&mut self, item: T) -> bool;
    fn as_queue_type(&self) -> &ItemQueue<T>;
    fn as_queue_type_mut(&mut self) -> &mut ItemQueue<T>;
}

pub struct NetAttempt {
    pub time: Instant,
    pub retries: usize,
}

impl NetAttempt {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
            retries: 0,
        }
    }

    #[allow(unused)]
    pub fn increment_retries(&mut self) {
        self.retries += 1;
        self.time = Instant::now();
    }
}

type ItemQueue<T> = VecDeque<T>;

pub struct NetQueue<T> {
    pub queue: ItemQueue<T>,
    pub attempts: VecDeque<NetAttempt>,
    pub buffer_wrap_index: Option<usize>,
}

impl NetQueue<Packet> {
    #[allow(unused)]
    pub fn get_retransmit_indices(&self) -> Vec<usize> {
        let iter = self.attempts.iter();
        iter.enumerate()
            .filter(|(_, ts)|
                (
                    (Instant::now() - ts.time) >= RETRANSMISSION_THRESHOLD_IN_MS)
                    || (ts.retries >= RETRY_THRESHOLD_IN_MS)
                    || (ts.retries >= RETRY_AGGRESSIVE_THRESHOLD_IN_MS)
                )
            .map(|(i, _)| i)
            .take(RETRANSMISSION_COUNT)
            .collect::<Vec<usize>>()
    }

}

impl<T> NetworkQueue<T> for NetQueue<T>
        where T: Sequenced+Debug+Clone {

    fn new() -> Self {
        NetQueue {
            queue:          ItemQueue::<T>::with_capacity(NETWORK_QUEUE_LENGTH),
            attempts:     VecDeque::<NetAttempt>::with_capacity(NETWORK_QUEUE_LENGTH),
            buffer_wrap_index: None
        }
    }

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

    fn clear(&mut self) {
        let Self {
            ref mut queue,
            ref mut attempts,
            ref mut buffer_wrap_index,
        } = *self;

        queue.clear();
        attempts.clear();
        *buffer_wrap_index = None;
    }

    fn remove(&mut self, pkt: &T) -> Option<T> {
        let result = {
            let search_space: Vec<&T> = self.as_queue_type_mut().iter().collect();
            search_space.as_slice().binary_search(&pkt)
        };
        match result {
            Err(_) => {
                warn!("Packet (Seq: {}) not present in queue! Was it removed already?", pkt.sequence_number());
                return None;
            }
            Ok(index) => {
                let pkt = self.as_queue_type_mut().remove(index).unwrap();
                self.attempts.remove(index);
                return Some(pkt);
            }
        }
    }

    /// Upon packet tx or rx, we must maintain linearly increasing sequence number order of items of type `T`.
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
    /// Because we may tx or rx `T`'s out-of-order even when wrapped, there are checks added below to safeguard
    /// against this. Primarily, they cover the cases where out-of-order insertion would transition the queue into a
    /// wrapped state from a non-wrapped state.
    ///
    /// boolean return value states whether or not the packet we are buffering is already present within the queue.
    fn buffer_item(&mut self, item: T) -> bool {
        let mut packet_exists: bool = false;
        let sequence = item.sequence_number();

        // Empty queue
        let opt_head_seq_num: Option<u64> = self.newest_seq_num();
        if opt_head_seq_num.is_none() {
            self.push_back(item);
            self.attempts.push_back(NetAttempt::new());
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
                    self.attempts.push_back(NetAttempt::new());
                    self.buffer_wrap_index = Some(self.len() - 1);
                } else {
                    self.push_front(item);
                    self.attempts.push_back(NetAttempt::new());
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
                    self.attempts.push_back(NetAttempt::new());
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
                self.attempts.push_back(NetAttempt::new());

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

impl<T> NetQueue<T> where T: Sequenced+Debug+Clone {

    /// Searching within the queue, but when we have no idea where to insert.
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
                if cfg!(test) {
                    self.as_queue_type_mut().insert(insertion_index, item.clone());
                    self.attempts.push_back(NetAttempt::new());
                }
            }
            if !(cfg!(test)) {
                self.as_queue_type_mut().insert(insertion_index, item);
                self.attempts.push_back(NetAttempt::new());
            }
        } else { exists = true; } // Packet is present in queue, hence None.
        return exists;
    }

    /// `seq_num` as a parameter specifies the starting sequence number to iterate over. Since packets can arrive
    /// out-of-order, the queue may be contiguous but not complete.
    /// Ex: Assume the next packet SN to process is 10, and the queue has buffered [10, 11, 12, 14, 16],
    /// the contiguous packet count would be 3.
    #[allow(unused)]
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
    pub tx_packets:       NetQueue<Packet>,                       // Back = Newest, Front = Oldest
    pub rx_packets:       NetQueue<Packet>,                       // Back = Newest, Front = Oldest
    pub rx_chat_messages: Option<NetQueue<BroadcastChatMessage>>, // Back = Newest, Front = Oldest;
                                                                 //     Messages are drained into the Client;
                                                                 //     Server does not use this structure.
}

impl NetworkManager {
    #[allow(unused)]
    pub fn new() -> Self {
        NetworkManager {
            statistics: NetworkStatistics::new(),
            tx_packets:  NetQueue::<Packet>::new(),
            rx_packets:  NetQueue::<Packet>::new(),
            rx_chat_messages: None,
        }
    }

    #[allow(unused)]
    pub fn with_message_buffering(self) -> NetworkManager {
        NetworkManager {
            statistics: self.statistics,
            tx_packets: self.tx_packets,
            rx_packets: self.rx_packets,
            rx_chat_messages:  Some(NetQueue::<BroadcastChatMessage>::new()),
        }
    }

    #[allow(unused)]
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

    #[allow(unused)]
    pub fn print_statistics(&self) {
        info!("Tx Successes: {}", self.statistics.tx_packets_success);
        info!("Tx Failures:  {}", self.statistics.tx_packets_failed);
    }

    #[allow(unused)]
    pub fn retransmit_expired_tx_packets(&mut self,
         udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>,
         addr: SocketAddr,
         confirmed_ack: Option<u64>,
         indices: &Vec<usize>) {

        let mut error_occurred = false;
        let mut failed_index = 0;

        // Retransmit all packets after that are still in the queue after RETRANSMISSION_THRESHOLD_IN_MS
        for &index in indices.iter() {

            let mut send_counter = 1;

            if let Some(ts) = self.tx_packets.attempts.get_mut(index) {
                ts.increment_retries();
                if ts.retries >= RETRY_AGGRESSIVE_THRESHOLD_IN_MS {
                    send_counter += 1;
                    ts.increment_retries()
                }
                if ts.retries >= RETRY_THRESHOLD_IN_MS {
                    send_counter += 1;
                    ts.increment_retries()
                }
            }

            if let Some(pkt) = self.tx_packets.queue.get_mut(index) {
                // `response_sequence` may have advanced since this was last queued
                pkt.set_response_sequence(confirmed_ack);
                trace!("[Retransmitting (Times={})] {:?}", send_counter, pkt);
                for _ in 0..send_counter {
                    netwayste_send!(udp_tx, (addr, (*pkt).clone()),
                                ("Could not retransmit packet to server: {:?}", pkt));
                }
            } else {
                error_occurred = true;
                failed_index = index;
                break;
            }

        }

        if error_occurred {
            // Panic during development, probably want to make this error later on
            panic!("ERROR: Index ({}) in attempt queue out-of-bounds in tx packets queue,
                            or perhaps `None`?:\n\t {:?}\n{:?}\n{:?}",
                    failed_index, indices, self.tx_packets.queue.len(), self.tx_packets.attempts.len());
        }
    }

    #[allow(unused)]
    pub fn tx_pop_front_with_count(&mut self, mut num_to_remove: usize) {
        if num_to_remove > self.tx_packets.len() {
            return;
        }

        while num_to_remove > 0 {
            self.tx_packets.as_queue_type_mut().pop_front();
            self.tx_packets.attempts.pop_front();
            num_to_remove -= 1;
        }
    }

}

#[derive(PartialEq, Debug, Clone)]
#[allow(dead_code)]
pub enum NetwaysteEvent {
    None,

    // Requests
    Connect(String, String),            // Player name, version
    Disconnect,
    List,
    ChatMessage(String),                // chat message
    NewRoom(String),                    // room name
    JoinRoom(String),                   // room name
    LeaveRoom,

    // Responses
    LoggedIn(String),                   // player is logged in -- (version)
    JoinedRoom(String),                 // player has joined the room
    PlayerList(Vec<String>),     // list of players in room or lobby with ping (ms)
    RoomList(Vec<(String, u64, bool)>), // (room name, # players, game has started?)
    LeftRoom,
    BadRequest(Option<String>),
    ServerError(Option<String>),

    // Updates
    ChatMessages(Vec<(String, String)>), // (player name, message)
    UniverseUpdate,                       // TODO add libconway stuff for current universe gen

}

impl NetwaysteEvent {

    #[allow(dead_code)]
    pub fn build_request_action_from_netwayste_event(nw_event: NetwaysteEvent, is_in_game: bool) -> RequestAction {
        match nw_event {
            NetwaysteEvent::None => {
                RequestAction::None
            }
            NetwaysteEvent::Connect(name, version) => {
                RequestAction::Connect{name: name, client_version: version}
            }
            NetwaysteEvent::Disconnect => {
                RequestAction::Disconnect
            }
            NetwaysteEvent::List => {
                // players or rooms
                if is_in_game {
                    RequestAction::ListPlayers
                } else {
                    // lobby
                    RequestAction::ListRooms
                }
            }
            NetwaysteEvent::ChatMessage(msg) => {
                RequestAction::ChatMessage(msg)
            }
            NetwaysteEvent::NewRoom(name) => {
                if !is_in_game {
                    RequestAction::NewRoom(name)
                } else {
                    debug!("Command failed: You are in a game");
                    RequestAction::None
                }
            }
            NetwaysteEvent::JoinRoom(name) => {
                if !is_in_game {
                    RequestAction::JoinRoom(name)
                } else {
                    debug!("Command failed: You are already in a game");
                    RequestAction::None
                }
            }
            NetwaysteEvent::LeaveRoom => {
                if is_in_game {
                    RequestAction::LeaveRoom
                } else {
                    debug!("Command failed: You are already in the lobby");
                    RequestAction::None
                }
            }
            _ => {
                panic!("Unexpected netwayste event during request action construction! {:?}", nw_event);
            }
        }
    }

    #[allow(dead_code)]
    pub fn build_netwayste_event_from_response_code(code: ResponseCode) -> NetwaysteEvent {
        match code {
            ResponseCode::LoggedIn(_cookie, server_version) => {
                NetwaysteEvent::LoggedIn(server_version)
            }
            ResponseCode::JoinedRoom(name) => {
                NetwaysteEvent::JoinedRoom(name)
            }
            ResponseCode::PlayerList(list) => {
                NetwaysteEvent::PlayerList(list)
            }
            ResponseCode::RoomList(list) => {
                NetwaysteEvent::RoomList(list)
            }
            ResponseCode::LeaveRoom => {
                NetwaysteEvent::LeftRoom
            }
            ResponseCode::BadRequest(opt_error) => {
                NetwaysteEvent::BadRequest(opt_error)
            }
            ResponseCode::ServerError( opt_error) => {
                NetwaysteEvent::ServerError(opt_error)
            }
            ResponseCode::Unauthorized(opt_error) => {
                NetwaysteEvent::BadRequest(opt_error)
            }
            _ => {
                panic!("Unexpected response code during netwayste event construction: {:?}", code);
            }
        }
    }

}
