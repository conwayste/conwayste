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
use std::io;
use std::net::{self, SocketAddr};
use std::str;
use std::result;
use std::time;
use std::collections::VecDeque;

use self::tokio_core::net::{UdpSocket, UdpCodec};
use self::tokio_core::reactor::Handle;
use self::bincode::{serialize, deserialize, Infinite};
use self::semver::{Version, SemVerError};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 12345;
const TIMEOUT_IN_SECONDS:    u64   = 5;
const PACKET_HISTORY_SIZE: usize = 15;

// For unit testing, I cover duplicate sequence numbers. The search returns Ok(index) on a slice with a matching value.
// Instead of returning that index, I return this much larger value and avoid insertion into the queues.
// (110 is the avg weight of an amino acid in daltons :] Much larger than our current queue size)
const MATCH_FOUND_SENTINEL: usize = 110;

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
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BroadcastChatMessage {
    pub chat_seq:    Option<u64>,   // Some(<number>) when sent to clients (starts at 0 for first
                                    // chat message sent to this client in this room); None when
                                    // internal to server
    pub player_name: String,
    pub message:     String,        // should not contain newlines
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        chats:           Option<Vec<BroadcastChatMessage>>,
        game_updates:    Option<Vec<GameUpdate>>,
        universe_update: UniUpdateType,
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
    fn sequence_number(&self) -> u64 {
        if let Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } = self {
            *sequence
        } else if let Packet::Response{ sequence, request_ack: _, code: _ } = self {
            *sequence
        } else {
            unimplemented!(); // TODO: Update/UpdateReply (likely needs sqn too)
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

impl Eq for Packet {

}

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

        // TODO check for Update/UpdateReply
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
                    println!("WARNING: error during packet deserialization: {:?}", e);
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

pub fn has_connection_timed_out(heartbeat: Option<time::Instant>) -> bool {
    match heartbeat {
        Some(heartbeat) =>  time::Instant::now() - heartbeat > time::Duration::from_secs(TIMEOUT_IN_SECONDS),
        None => false
    }
}

pub struct NetworkStatistics {
    packets_tx_failed: u64,
    packets_tx_success: u64,
    keep_alive_tx_failed: u64,
    keep_alive_tx_success: u64,
}

impl NetworkStatistics {
    fn new() -> Self {
        NetworkStatistics {
            packets_tx_success: 0,
            packets_tx_failed: 0,
            keep_alive_tx_failed: 0,
            keep_alive_tx_success: 0
        }
    }

    pub fn inc_tx_packets_failed(&mut self) {
        self.packets_tx_failed += 1;
    }

    pub fn inc_tx_packets_success(&mut self) {
        self.packets_tx_success += 1;
    }

    pub fn inc_tx_keep_alive_failed(&mut self) {
        self.keep_alive_tx_failed += 1;
    }

    pub fn inc_tx_keep_alive_success(&mut self) {
        self.keep_alive_tx_success += 1;
    }
}

pub struct NetworkManager {
    pub statistics:     NetworkStatistics,
    pub tx_packets:     VecDeque<Packet>, // Back == Newest, Front == Oldest
    rx_packets:         VecDeque<Packet>, // Back == Newest, Front == Oldest
    rx_buffer_wrap_index: Option<usize>,  // Index in rx_packets that sequence number wrap occurs
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            statistics: NetworkStatistics::new(),
            tx_packets:  VecDeque::<Packet>::with_capacity(PACKET_HISTORY_SIZE),
            rx_packets:  VecDeque::<Packet>::with_capacity(PACKET_HISTORY_SIZE),
            rx_buffer_wrap_index: None,
        }
    }

    pub fn head_of_tx_packet_queue(&self) -> Option<&Packet> {
        self.tx_packets.back()
    }

    pub fn head_of_rx_packet_queue(&self) -> Option<&Packet> {
        self.rx_packets.back()
    }

    pub fn tail_of_rx_packet_queue(&self) -> Option<&Packet> {
        self.rx_packets.front()
    }

    fn newest_tx_packet_seq_num(&self) -> Option<u64> {
        let opt_newest_packet = self.head_of_tx_packet_queue();

        if opt_newest_packet.is_some() {
            let newest_packet = opt_newest_packet.unwrap();
            Some(newest_packet.sequence_number())
        } else { None }
                // Queue is empty
    }

    fn newest_rx_packet_seq_num(&self) -> Option<u64> {
        let opt_newest_packet = self.head_of_rx_packet_queue();

        if opt_newest_packet.is_some() {
            let newest_packet = opt_newest_packet.unwrap();
            Some(newest_packet.sequence_number())
        } else { None }
                // Queue is empty
    }

    fn oldest_rx_packet_seq_num(&self) -> Option<u64> {
        let opt_oldest_packet = self.tail_of_rx_packet_queue();

        if opt_oldest_packet.is_some() {
            let oldest_packet = opt_oldest_packet.unwrap();
            Some(oldest_packet.sequence_number())
        } else { None }
                // Queue is empty
    }

    /// This will keep the specified queue under the PACKET_HISTORY_SIZE limit.
    /// The TX queue needs to ensure a spot is open if we're at capacity.
    fn discard_older_packets(&mut self, is_tx_queue: bool) {
        let queue = match is_tx_queue {
            true => &mut self.tx_packets,
            false => &mut self.rx_packets,
        };
        let queue_size = queue.len();
        if queue_size >= PACKET_HISTORY_SIZE {
            for _ in 0..(queue_size-PACKET_HISTORY_SIZE) {
                queue.pop_front();
            }

            // Keep 1 spot empty for upcoming enqueue
            if is_tx_queue {
                queue.pop_front();
            }
        }
    }

    /// Determines where in the rx queue to insert the new packet based on sequence number
    /// This should cover only within the RX queue and not at the edges (front or back).
    /// We accomplish this by splitting the VecDequeue into a slice tuple and then binary searching on each slice.
    /// Small note: The splitting of VecDequeue is into its 'front' and 'back' halves, based on how 'push_front' and 'push_back' were used.
    fn find_rx_insertion_index(&self, sequence: u64) -> Option<usize> {
        let (front_slice, back_slice) = self.rx_packets.as_slices();
        let f_result = front_slice.binary_search(&Packet::Request{sequence, response_ack: None, cookie: None, action: RequestAction::None});
        let b_result = back_slice.binary_search(&Packet::Request{sequence, response_ack: None, cookie: None, action: RequestAction::None});

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

    fn linear_queue_search(&self, start: usize, end: usize, sequence: u64) -> Option<usize> {
        let search_space: Vec<&Packet> = self.rx_packets.iter().skip(start).take(end).collect();
        let result = search_space.as_slice().binary_search(&&Packet::Request{sequence, response_ack: None, cookie: None, action: RequestAction::None});
        match result {
            Err(loc) => Some(loc + start),
            #[cfg(test)]
            Ok(_) => Some(MATCH_FOUND_SENTINEL),
            #[cfg(not(test))]
            Ok(_) => None,
        }
    }

    /// As we buffer new packets, we'll want to throw away the older packets.
    /// We must be careful to ensure that we do not throw away packets that have
    /// not yet been acknowledged by the end-point.
    pub fn buffer_tx_packet(&mut self, packet: Packet) {

        let sequence = match packet {
            Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } => sequence,
            Packet::Response{ sequence, request_ack: _, code: _ } => sequence,
            _ => return
        };

        let opt_head_seq_num: Option<u64> = self.newest_tx_packet_seq_num();

        if opt_head_seq_num.is_none() {
            self.tx_packets.push_back(packet);
            return;
        }

        let head_seq_num = opt_head_seq_num.unwrap(); // unwrap safe

        self.discard_older_packets(true);

        // Current packet is newer, and we're adding in sequential order
        if head_seq_num < sequence && head_seq_num+1 == sequence {
            self.tx_packets.push_back(packet);
        }
    }

    /// Compares sequence against the front and back of the queue.
    /// Sequence must be >=, or <, what we're comparing against
    fn is_seq_about_to_wrap(&self, sequence: u64, oldest_seq_num: u64, newest_seq_num: u64) -> bool {
        if self.rx_buffer_wrap_index.is_none() {
            if sequence >= oldest_seq_num && sequence >= newest_seq_num {
                self.is_seq_sufficiently_far_away(sequence, oldest_seq_num)
                && self.is_seq_sufficiently_far_away(sequence, newest_seq_num)
            } else {
                self.is_seq_sufficiently_far_away(oldest_seq_num, sequence)
                && self.is_seq_sufficiently_far_away(newest_seq_num, sequence)
            }
        } else {
            false
        }
    }

    /// I've deemed 'far away' to mean the half of the max value of the type.
    fn is_seq_sufficiently_far_away(&self, a: u64, b: u64) -> bool {
        static HALFWAYPOINT: u64 = u64::max_value()/2;
        a - b > HALFWAYPOINT
    }

    /// Upon receipt, we must maintain linearly increasing sequence number order of received packets.
    ///
    /// This also handles the case where the sequence number numerically wraps.
    /// `rx_buffer_wrap_index` is maintained to denote the queue index at which the numerical wrap occurs.
    /// In general, it is used to determine if a wrap has occurred yet, and if so, which half of the queue to insert into.
    ///
    /// For sequence numbers of type 'Byte':
    ///     [253, 254, 1, 2, 4]
    ///                ^ wrapping index
    ///     Inserting '255' performs a linear search on the subset [253, 254] only.
    ///     After insertion, the wrapping index increments since we modified the left half.
    ///     [253, 254, 255, 1, 2, 4]
    ///                     ^ wrapping index
    ///     Inserting '3' performs a linear search on [1, 2, 4]. Does not impact wrapping index.
    pub fn buffer_rx_packet(&mut self, packet: Packet) {
        let sequence = packet.sequence_number();

        // Empty queue
        let opt_head_seq_num: Option<u64> = self.newest_rx_packet_seq_num();
        if opt_head_seq_num.is_none() {
            self.rx_packets.push_back(packet);
            return;
        }
        let opt_tail_seq_num: Option<u64> = self.oldest_rx_packet_seq_num();
        let newest_seq_num = opt_head_seq_num.unwrap();
        let oldest_seq_num = opt_tail_seq_num.unwrap();

        if sequence < oldest_seq_num {
            if newest_seq_num == u64::max_value() {
                if self.is_seq_about_to_wrap(sequence, oldest_seq_num, newest_seq_num) {
                    self.rx_packets.push_back(packet);
                    if let Some(buffer_wrap_index) = self.rx_buffer_wrap_index {
                        self.rx_buffer_wrap_index = Some(buffer_wrap_index - 1);
                    } else {
                        self.rx_buffer_wrap_index = Some(self.rx_packets.len() - 1);
                    }
                } else {
                    self.rx_packets.push_front(packet);
                }
            } else if sequence > newest_seq_num && self.rx_buffer_wrap_index.is_some() {
                // Possible that we get a before-wrap sequence number
                if self.is_seq_sufficiently_far_away(sequence, newest_seq_num) {
                    if let Some(buffer_wrap_index) = self.rx_buffer_wrap_index {
                        let insertion_index = self.linear_queue_search(0, buffer_wrap_index, sequence);
                        self.rx_buffer_wrap_index = Some(buffer_wrap_index + 1);
                        if let Some(insertion_index) = insertion_index {
                            self.rx_packets.insert(insertion_index, packet);
                        }
                    }
                } else {
                    self.rx_packets.push_back(packet);
                }
            } else if sequence < newest_seq_num {

                let insertion_index: Option<usize>;

                if self.is_seq_about_to_wrap(sequence, oldest_seq_num, newest_seq_num) {
                    insertion_index = Some(self.rx_packets.len());
                    self.rx_buffer_wrap_index = insertion_index;
                } else if let Some(buffer_wrap_index) = self.rx_buffer_wrap_index {
                    insertion_index = self.linear_queue_search(buffer_wrap_index, self.rx_packets.len(), sequence);
                } else {
                    insertion_index = self.find_rx_insertion_index(sequence);
                }
                if let Some(insertion_index) = insertion_index {
                    self.rx_packets.insert(insertion_index, packet);
                }

            } else {
                self.rx_packets.push_front(packet);
                if self.rx_buffer_wrap_index.is_some() {
                    self.rx_buffer_wrap_index = Some(self.rx_buffer_wrap_index.unwrap() + 1);
                }
            }
        } else {
            if sequence < newest_seq_num {
                let insertion_index: Option<usize> = self.find_rx_insertion_index(sequence);
                if let Some(insertion_index) = insertion_index {
                    self.rx_packets.insert(insertion_index, packet);
                }
            } else {
                let insertion_index: Option<usize>;

                // Sequence Number has wrapped, insert from 0 to wrap index.
                if let Some(buffer_wrap_index) = self.rx_buffer_wrap_index {
                    insertion_index = self.linear_queue_search(0, buffer_wrap_index, sequence);
                    self.rx_buffer_wrap_index = Some(buffer_wrap_index + 1);
                } else {
                    if self.is_seq_about_to_wrap(sequence, oldest_seq_num, newest_seq_num) {
                        insertion_index = Some(0);
                        self.rx_buffer_wrap_index = Some(1);
                    } else {
                        // No wrap yet, and not about to either, use a binary search
                        insertion_index = self.find_rx_insertion_index(sequence);
                    }
                }
                if let Some(insertion_index) = insertion_index {
                    if insertion_index != MATCH_FOUND_SENTINEL {
                        #[cfg(test)]
                        self.rx_packets.insert(insertion_index, packet);
                    }
                    #[cfg(not(test))]
                    self.rx_packets.insert(insertion_index, packet);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_discard_older_packets_empty_queue() {
        let mut nm = NetworkManager::new();

        nm.discard_older_packets(true);
        nm.discard_older_packets(false);
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

        nm.discard_older_packets(true);
        assert_eq!(nm.tx_packets.len(), 3);

        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());

        nm.discard_older_packets(false);
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

        for _ in 0..PACKET_HISTORY_SIZE {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), PACKET_HISTORY_SIZE);
        nm.discard_older_packets(true);
        assert_eq!(nm.tx_packets.len(), PACKET_HISTORY_SIZE-1);

        for _ in 0..PACKET_HISTORY_SIZE {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), PACKET_HISTORY_SIZE);
        nm.discard_older_packets(false);
        assert_eq!(nm.rx_packets.len(), PACKET_HISTORY_SIZE);
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

        for _ in 0..PACKET_HISTORY_SIZE+10 {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), PACKET_HISTORY_SIZE+10);
        nm.discard_older_packets(true);
        assert_eq!(nm.tx_packets.len(), PACKET_HISTORY_SIZE-1);

        for _ in 0..PACKET_HISTORY_SIZE+5 {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), PACKET_HISTORY_SIZE+5);
        nm.discard_older_packets(false);
        assert_eq!(nm.rx_packets.len(), PACKET_HISTORY_SIZE);
    }

    #[test]
    fn test_buffer_tx_packet_queue_is_empty() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.buffer_tx_packet(pkt);
        assert_eq!(nm.tx_packets.len(), 1);
    }

    #[test]
    fn test_buffer_tx_packet_sequence_number_reused() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.buffer_tx_packet(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };

        nm.buffer_tx_packet(pkt);
        let pkt = nm.tx_packets.back().unwrap();
        if let Packet::Request { sequence: _, response_ack: _, cookie: _, action } = pkt {
            assert_eq!(*action, RequestAction::None);
        }
    }

    #[test]
    fn test_buffer_tx_packet_basic_sequencing() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.buffer_tx_packet(pkt);
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.buffer_tx_packet(pkt);
        assert_eq!(nm.tx_packets.len(), 2);
    }

    #[test]
    fn test_buffer_tx_packet_newer_packet_has_smaller_sequence_number() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.buffer_tx_packet(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.buffer_tx_packet(pkt);
        assert_eq!(nm.tx_packets.len(), 1);

        let pkt = nm.tx_packets.back().unwrap();
        if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
            assert_eq!(*sequence, 1);
        }
    }

    #[test]
    fn test_buffer_tx_packet_max_queue_limit_maintained() {
        let mut nm = NetworkManager::new();
        for index in 0..PACKET_HISTORY_SIZE+5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_tx_packet(pkt);
        }

        let mut iter =  nm.tx_packets.iter();
        for index in 5..PACKET_HISTORY_SIZE+5 {
            let pkt = iter.next().unwrap();
            if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
                assert_eq!(*sequence, index as u64);
            }
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_contiguous_ascending() {
        let mut nm = NetworkManager::new();
        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_contiguous_descending() {
        let mut nm = NetworkManager::new();
        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_sequential_gap_ascending() {
        let mut nm = NetworkManager::new();
        // TODO Replace with (x,y).step_by(z) once stable
        for index in [0,2,4,6,8,10].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_sequential_gap_descending() {
        let mut nm = NetworkManager::new();
        for index in [0,2,4,6,8,10].iter().rev() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_random() {
        let mut nm = NetworkManager::new();
        for index in [5, 2, 9, 1, 0, 8, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in [0, 1, 2, 5, 6, 8, 9].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_butterfly_pattern() {
        let mut nm = NetworkManager::new();
        // This one is fun because it tests the internal edges of (front_slice and back_slice)
        for index in [0, 10, 1, 9, 2, 8, 3, 7, 4, 6, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_repetition() {
        let mut nm = NetworkManager::new();
        for index in [0, 0, 0, 0, 1, 2, 2, 2, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        for index in [0, 1, 2, 5].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_sequential_then_pseudorandom_then_sequential() {
        let mut nm = NetworkManager::new();

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in 13..20 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_reverse_sequential_then_random_then_reverse_sequential() {
        let mut nm = NetworkManager::new();

        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in (13..20).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_wrapping_case() {
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
            nm.buffer_rx_packet(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_basic_wrapping_case_then_out_of_order() {
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
            nm.buffer_rx_packet(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        for index in [5, 0, 4, 1, 3, 2].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4, 5].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrapping_case_everything_out_of_order() {
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

        let input_order = [max_minus_4, two, max_minus_1, max_minus_5, u64_max, three, max_minus_2, zero, max_minus_3, one];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_max_sequence_number_arrives_after_a_wrap() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_2, max_minus_1, u64_max, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_oldest_sequence_number_arrived_last() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrap_occurs_with_two_item_queue() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrap_occurs_with_two_item_queue_in_reverse() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrapping_case_max_arrives_first() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrapping_case_sequence_number_descending() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_rx_packet_advanced_wrapping_case_sequence_number_alternating() {
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
            nm.buffer_rx_packet(pkt);
        }

        let mut iter = nm.rx_packets.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }
}
