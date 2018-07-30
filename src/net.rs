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

use self::tokio_core::net::{UdpSocket, UdpCodec};
use self::tokio_core::reactor::Handle;
use self::bincode::{serialize, deserialize, Infinite};
use self::semver::{Version, SemVerError};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 12345;

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
                let local: SocketAddr = format!("{}:{}", "127.0.0.1", PORT.to_string()).parse().unwrap();
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
