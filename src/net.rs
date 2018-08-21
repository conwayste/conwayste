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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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
    if let Some(heartbeat) = heartbeat {
        (time::Instant::now() - heartbeat) > time::Duration::from_secs(TIMEOUT_IN_SECONDS)
    } else { false }
}

pub struct NetworkStatistics {
    pub tx_packets_failed: u64,
    pub tx_packets_success: u64,
    pub tx_keep_alive_failed: u64,
    pub tx_keep_alive_success: u64,
}

impl NetworkStatistics {
    fn new() -> Self {
        NetworkStatistics {
            tx_packets_failed: 0,
            tx_packets_success: 0,
            tx_keep_alive_failed: 0,
            tx_keep_alive_success: 0
        }
    }
}

pub struct NetworkManager {
    pub statistics:     NetworkStatistics,
    pub tx_packets:     VecDeque<Packet>, // Back == Newest, Front == Oldest
    rx_packets:     VecDeque<Packet>, // Back == Newest, Front == Oldest
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            statistics: NetworkStatistics::new(),
            tx_packets:  VecDeque::<Packet>::with_capacity(PACKET_HISTORY_SIZE),
            rx_packets:  VecDeque::<Packet>::with_capacity(PACKET_HISTORY_SIZE),
        }
    }

    pub fn newest_tx_packet_in_queue(&self) -> Option<&Packet> {
        self.tx_packets.back()
    }

    /// The TX Packet queue must only contain Request packets.
    fn newest_tx_packet_seq_num(&self) -> Option<u64> {
        if let Some(newest_packet) = self.newest_tx_packet_in_queue() {
            if let Packet::Request{ sequence: newest_sequence, response_ack: _, cookie: _, action: _ } = *newest_packet {
                Some(newest_sequence)
            } else { panic!("Found something other than a `Request` packet in the buffer: {:?}", newest_packet) }
                    // Somehow not a Request packet. Panic during development XXX
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

    /// As we buffer new packets, we'll want to throw away the older packets.
    /// We must be careful to ensure that we do not throw away packets that have
    /// not yet been acknowledged by the end-point.
    pub fn buffer_tx_packet(&mut self, packet: Packet) {
        if let Packet::Request{ sequence, response_ack: _, cookie: _, action: _ } = packet {

            if let Some(newest_seq_num) = self.newest_tx_packet_seq_num() {
                self.discard_older_packets(true);

                // Current packet is newer, and we're adding in sequential order
                if newest_seq_num < sequence && newest_seq_num+1 == sequence {
                    self.tx_packets.push_back(packet);
                }
            } else {
                self.tx_packets.push_back(packet);
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
    fn test_buffer_tx_packet_not_a_request_packet() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Response {
            sequence: 0,
            request_ack: None,
            code: ResponseCode::OK
        };

        nm.buffer_tx_packet(pkt);
        assert_eq!(nm.tx_packets.len(), 0);
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
}