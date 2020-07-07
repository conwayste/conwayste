#![recursion_limit = "300"] // The select!{...} macro hits the default 128 limit

/*
 * Herein lies a networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2018-2020 The Conwayste Developers
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

#[macro_use]
extern crate log;

#[macro_use]
mod net;
mod utils;

#[cfg(test)]
#[macro_use]
extern crate proptest;

use netwayste::net::{
    bind, get_version, has_connection_timed_out, BroadcastChatMessage, NetwaystePacketCodec, NetworkManager,
    NetworkQueue, Packet, RequestAction, ResponseCode, RoomList, UniUpdateType, DEFAULT_HOST, DEFAULT_PORT, VERSION,
};
use netwayste::utils::{LatencyFilter, PingPong};

use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::io::{self, ErrorKind, Write};
use std::net::SocketAddr;
use std::process::exit;
use std::time::{self, Duration, Instant};

use chrono::Local;
use clap::{App, Arg};
use futures as F;
use log::LevelFilter;
use rand::RngCore;
use semver::Version;
use tokio::time as TT;
use tokio_util::udp::UdpFramed;
use F::prelude::*;
use F::select;

pub const TICK_INTERVAL_IN_MS: u64 = 10;
pub const NETWORK_INTERVAL_IN_MS: u64 = 100; // Arbitrarily chosen
pub const HEARTBEAT_INTERVAL_IN_MS: u64 = 1000; // Arbitrarily chosen
pub const MAX_ROOM_NAME: usize = 16;
pub const MAX_NUM_CHAT_MESSAGES: usize = 128;
pub const MAX_AGE_CHAT_MESSAGES: usize = 60 * 5; // seconds
pub const SERVER_ID: PlayerID = PlayerID(u64::max_value()); // 0xFFFF....FFFF

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
pub struct PlayerID(pub u64);

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
pub struct RoomID(pub u64);

impl fmt::Display for PlayerID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

impl fmt::Display for RoomID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Player {
    pub player_id:      PlayerID,
    pub cookie:         String,
    pub addr:           SocketAddr,
    pub name:           String,
    pub request_ack:    Option<u64>, // The next number we expect is request_ack + 1
    pub next_resp_seq:  u64, // This is the sequence number for the Response packet the Server sends to the Client
    pub game_info:      Option<PlayerInGameInfo>, // none means in lobby
    pub last_received:  time::Instant, // Time of last message received from player
    pub latency_filter: LatencyFilter, // Latency information
}

// info for a player as it relates to a game/room
#[derive(PartialEq, Debug, Clone)]
pub struct PlayerInGameInfo {
    room_id:          RoomID,
    chat_msg_seq_num: Option<u64>, // Server has confirmed the client has received messages up to this value.
                                   //XXX PlayerGenState ID within Universe
                                   //XXX update statuses
}

impl Player {
    pub fn increment_response_seq_num(&mut self) -> u64 {
        let old_seq = self.next_resp_seq;
        self.next_resp_seq += 1;
        old_seq
    }

    // Update the Server's record of what chat messsage the player has obtained.
    // If the player is in a game, and the player has seen newer chat messages since the last time
    // they updated us on what messages they had, save their sequence number.
    pub fn update_chat_seq_num(&mut self, opt_chat_seq_num: Option<u64>) {
        if self.game_info.is_none() {
            return;
        }
        let game_info: &mut PlayerInGameInfo = self.game_info.as_mut().unwrap();

        if game_info.chat_msg_seq_num.is_none() || game_info.chat_msg_seq_num < opt_chat_seq_num {
            game_info.chat_msg_seq_num = opt_chat_seq_num;
        }
    }

    // If the player has chatted, we'll return Some(N),
    // where N is the last chat message the player has
    // notified the Server it got.
    // Otherwise, None
    pub fn get_confirmed_chat_seq_num(&self) -> Option<u64> {
        if self.game_info.is_none() {
            return None;
        }

        if let Some(ref game_info) = self.game_info {
            return game_info.chat_msg_seq_num;
        }
        return None;
    }

    // Allow dead_code for unit testing
    #[cfg(test)]
    pub fn has_chatted(&self) -> bool {
        if self.game_info.is_none() {
            return false;
        }

        if let Some(ref game_info) = self.game_info {
            return game_info.chat_msg_seq_num.is_some();
        }
        return false;
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct ServerChatMessage {
    pub seq_num:     u64, // sequence number
    pub player_id:   PlayerID,
    pub player_name: String,
    pub message:     String,
    pub timestamp:   Instant,
}

#[derive(Clone, PartialEq)]
pub struct Room {
    pub room_id:        RoomID,
    pub name:           String,
    pub player_ids:     Vec<PlayerID>,
    pub game_running:   bool,
    pub universe:       u64, // Temp until we integrate
    pub latest_seq_num: u64,
    pub messages:       VecDeque<ServerChatMessage>, // Front == Oldest, Back == Newest
}

pub struct ServerState {
    pub tick:        usize,
    pub players:     HashMap<PlayerID, Player>,
    pub player_map:  HashMap<String, PlayerID>, // map cookie to player ID
    pub rooms:       HashMap<RoomID, Room>,
    pub room_map:    HashMap<String, RoomID>, // map room name to room ID
    pub network_map: HashMap<PlayerID, NetworkManager>, // map Player ID to Player's network data
}

//////////////// Utilities ///////////////////////

pub fn new_cookie() -> String {
    let mut buf = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut buf);
    let config = base64::Config::new(base64::CharacterSet::UrlSafe, false);
    base64::encode_config(&buf, config)
}

/*
*  Entity (Player/Room) IDs are comprised of:
*      1) Current timestamp (lower 24 bits)
*      2) A random salt
*
*       64 bits total
*  _________________________
*  |  32 bits  |  32 bits  |
*  | timestamp | rand_salt |
*  |___________|___________|
*/
pub fn new_uuid() -> u64 {
    let hash: u64;

    let mut timestamp: u64 = time::Instant::now().elapsed().as_secs().into();
    timestamp = timestamp & 0xFFFFFFFF;

    let mut rand_salt: u64 = rand::thread_rng().next_u32().into();
    rand_salt = rand_salt & 0xFFFFFFFF;

    hash = (timestamp << 32) | rand_salt;
    hash
}

pub fn validate_client_version(client_version: String) -> bool {
    let server_version = get_version();

    // Client cannot be newer than server
    server_version >= Version::parse(&client_version)
}

impl ServerChatMessage {
    pub fn new(id: PlayerID, name: String, msg: String, seq_num: u64) -> Self {
        ServerChatMessage {
            player_id:   id,
            player_name: name,
            message:     msg,
            seq_num:     seq_num,
            timestamp:   time::Instant::now(),
        }
    }
}

impl Room {
    /// Instantiates a `Room` with the provided `name` and adds
    /// the players (via `player_ids`) immediately to it.
    pub fn new(name: String, player_ids: Vec<PlayerID>) -> Self {
        Room {
            room_id:        RoomID(new_uuid()),
            name:           name,
            player_ids:     player_ids,
            game_running:   false,
            universe:       0,
            messages:       VecDeque::<ServerChatMessage>::with_capacity(MAX_NUM_CHAT_MESSAGES),
            latest_seq_num: 0,
        }
    }

    /// The room message queue cannot exceed `MAX_NUM_CHAT_MESSAGES` so we
    /// will dequeue the oldest messages until we are within limits.
    pub fn discard_older_messages(&mut self) {
        let queue_size = self.messages.len();
        if queue_size >= MAX_NUM_CHAT_MESSAGES {
            for _ in 0..(queue_size - MAX_NUM_CHAT_MESSAGES + 1) {
                self.messages.pop_front();
            }
        }
    }

    pub fn has_players(&mut self) -> bool {
        !self.player_ids.is_empty()
    }

    /// Increments the room's latest sequence number
    pub fn increment_seq_num(&mut self) -> u64 {
        self.latest_seq_num += 1;
        self.latest_seq_num
    }

    /// Adds a new message to the room message queue
    pub fn add_message(&mut self, new_message: ServerChatMessage) {
        self.messages.push_back(new_message);
    }

    /// Gets the oldest message in the room message queue
    pub fn get_oldest_msg(&self) -> Option<&ServerChatMessage> {
        return self.messages.front();
    }

    /// Gets the newest message in the room message queue
    pub fn get_newest_msg(&self) -> Option<&ServerChatMessage> {
        return self.messages.back();
    }

    /// This function retrieves the number of messages that have
    /// already been acknowledged by the client. One use of this is
    /// to only send unread messages.
    pub fn get_message_skip_count(&self, chat_msg_seq_num: u64) -> u64 {
        let opt_newest_msg = self.get_newest_msg();
        if opt_newest_msg.is_none() {
            return 0;
        }
        let newest_msg = opt_newest_msg.unwrap();

        let opt_oldest_msg = self.get_oldest_msg();
        if opt_oldest_msg.is_none() {
            return 0;
        }
        let oldest_msg = opt_oldest_msg.unwrap();

        // Skip over these messages since we've already acked them
        let amount_to_consume: u64 = if chat_msg_seq_num >= oldest_msg.seq_num {
            ((chat_msg_seq_num - oldest_msg.seq_num) + 1) % (MAX_NUM_CHAT_MESSAGES as u64)
        } else if chat_msg_seq_num < oldest_msg.seq_num && oldest_msg.seq_num != newest_msg.seq_num {
            // Sequence number has wrapped
            (<u64>::max_value() - oldest_msg.seq_num) + chat_msg_seq_num + 1
        } else {
            0
        };

        return amount_to_consume;
    }

    /// Send a message to all players in room notifying that an event took place.
    pub fn broadcast(&mut self, event: String) {
        self.discard_older_messages();
        let seq_num = self.increment_seq_num();
        self.add_message(ServerChatMessage::new(SERVER_ID, "Server".to_owned(), event, seq_num));
    }
}

impl ServerState {
    pub fn get_player(&self, player_id: PlayerID) -> &Player {
        let opt_player = self.players.get(&player_id);

        if opt_player.is_none() {
            panic!("player_id: {} could not be found!", player_id);
        }

        opt_player.unwrap()
    }

    pub fn get_player_mut(&mut self, player_id: PlayerID) -> &mut Player {
        let opt_player = self.players.get_mut(&player_id);

        if opt_player.is_none() {
            panic!("player_id: {} could not be found!", player_id);
        }

        opt_player.unwrap()
    }

    pub fn get_room_id(&self, player_id: PlayerID) -> Option<RoomID> {
        let player = self.get_player(player_id);
        if player.game_info == None {
            return None;
        };

        Some(player.game_info.as_ref().unwrap().room_id) // unwrap ok because of test above
    }

    pub fn get_room_mut(&mut self, player_id: PlayerID) -> Option<&mut Room> {
        let opt_room_id = self.get_room_id(player_id);

        if opt_room_id.is_none() {
            return None;
        }
        self.rooms.get_mut(&opt_room_id.unwrap())
    }

    pub fn get_room(&self, player_id: PlayerID) -> Option<&Room> {
        let opt_room_id = self.get_room_id(player_id);

        if opt_room_id.is_none() {
            return None;
        }
        self.rooms.get(&opt_room_id.unwrap())
    }

    pub fn list_players(&self, player_id: PlayerID) -> ResponseCode {
        let opt_room = self.get_room(player_id);
        if opt_room.is_none() {
            return ResponseCode::BadRequest {
                error_msg: "cannot list players because in lobby.".to_owned(),
            };
        }
        let room = opt_room.unwrap();

        let mut players = vec![];
        self.players.values().for_each(|p| {
            if room.player_ids.contains(&p.player_id) {
                players.push(p.name.clone());
            }
        });

        return ResponseCode::PlayerList { players };
    }

    pub fn handle_chat_message(&mut self, player_id: PlayerID, msg: String) -> ResponseCode {
        let player_in_game = self.is_player_in_game(player_id);

        if !player_in_game {
            return ResponseCode::BadRequest {
                error_msg: format!("Player {} has not joined a game.", player_id),
            };
        }

        // We're borrowing self mutably below, so let's grab this now
        let player_name = {
            let player = self.players.get(&player_id);
            player.unwrap().name.clone()
        };

        // User is in game, Server needs to broadcast this to Room
        let opt_room = self.get_room_mut(player_id);

        if opt_room.is_none() {
            return ResponseCode::BadRequest {
                error_msg: format!("Player \"{}\" should be in a room! None found.", player_id),
            };
        }

        let room = opt_room.unwrap();
        let seq_num = room.increment_seq_num();

        room.discard_older_messages();
        room.add_message(ServerChatMessage::new(player_id, player_name, msg, seq_num));

        return ResponseCode::OK;
    }

    pub fn list_rooms(&mut self) -> ResponseCode {
        let mut rooms = vec![];
        self.rooms.values().for_each(|gs| {
            let room_details = RoomList {
                room_name:    gs.name.clone(),
                player_count: gs.player_ids.len() as u8,
                in_progress:  gs.game_running,
            };
            rooms.push(room_details);
        });
        ResponseCode::RoomList { rooms }
    }

    /// Creates a new room. Does _not_ check whether it already exists!
    pub fn new_room(&mut self, name: String) -> RoomID {
        let room = Room::new(name.clone(), vec![]);
        let id = room.room_id;

        self.room_map.insert(name, room.room_id);
        self.rooms.insert(room.room_id, room);
        id
    }

    pub fn create_new_room(&mut self, opt_player_id: Option<PlayerID>, room_name: String) -> ResponseCode {
        // validate length
        if room_name.len() > MAX_ROOM_NAME {
            return ResponseCode::BadRequest {
                error_msg: format!("room name too long; max {} characters", MAX_ROOM_NAME),
            };
        }

        if let Some(player_id) = opt_player_id {
            if self.is_player_in_game(player_id) {
                return ResponseCode::BadRequest {
                    error_msg: "cannot create room because in-game".to_owned(),
                };
            }
        }

        // Create room if the room name is not already taken
        if !self.room_map.get(&room_name).is_some() {
            self.new_room(room_name);

            return ResponseCode::OK;
        } else {
            return ResponseCode::BadRequest {
                error_msg: format!("room name already in use"),
            };
        }
    }

    pub fn join_room(&mut self, player_id: PlayerID, room_name: &str) -> ResponseCode {
        let already_playing = self.is_player_in_game(player_id);
        if already_playing {
            return ResponseCode::BadRequest {
                error_msg: "cannot join game because in-game".to_owned(),
            };
        }

        let player: &mut Player = self.players.get_mut(&player_id).unwrap();

        // TODO replace loop with `get_key_value` once it reaches stable. Same thing with `leave_room` algorithm
        for ref mut gs in self.rooms.values_mut() {
            if gs.name == room_name {
                gs.player_ids.push(player_id);
                player.game_info = Some(PlayerInGameInfo {
                    room_id:          gs.room_id.clone(),
                    chat_msg_seq_num: None,
                });
                return ResponseCode::JoinedRoom {
                    room_name: room_name.to_owned(),
                };
            }
        }
        ResponseCode::BadRequest {
            error_msg: format!("no room named {:?}", room_name),
        }
    }

    pub fn leave_room(&mut self, player_id: PlayerID) -> ResponseCode {
        let already_playing = self.is_player_in_game(player_id);
        if !already_playing {
            return ResponseCode::BadRequest {
                error_msg: "cannot leave game because in lobby".to_owned(),
            };
        }

        let player: &mut Player = self.players.get_mut(&player_id).unwrap();
        {
            let room_id = &player.game_info.as_ref().unwrap().room_id; // unwrap ok because of test above
            for ref mut gs in self.rooms.values_mut() {
                if gs.room_id == *room_id {
                    // remove player_id from room's player_ids
                    gs.player_ids.retain(|&p_id| p_id != player.player_id);
                    break;
                }
            }
        }
        player.game_info = None;

        return ResponseCode::LeaveRoom;
    }

    pub fn remove_player(&mut self, player_id: PlayerID, player_cookie: &str) {
        if self.is_player_in_game(player_id) {
            let player = self.get_player(player_id);
            let broadcast_msg = format!("Player {} has left.", player.name);
            let room: &mut Room = self.get_room_mut(player_id).unwrap(); // safe because in game check verifies room's existence
            room.broadcast(broadcast_msg);
            let _left = self.leave_room(player_id); // Ignore return since we don't care
        }
        self.player_map.remove(player_cookie);
        self.players.remove(&player_id);
    }

    pub fn handle_disconnect(&mut self, player_id: PlayerID) -> ResponseCode {
        let player = self.get_player(player_id);
        let player_cookie = player.cookie.clone();
        self.remove_player(player_id, &player_cookie);

        ResponseCode::OK
    }

    // not used for connect
    pub fn process_request_action(&mut self, player_id: PlayerID, action: RequestAction) -> ResponseCode {
        match action {
            RequestAction::Disconnect => {
                return self.handle_disconnect(player_id);
            }
            RequestAction::KeepAlive { latest_response_ack: _ } => {
                return ResponseCode::OK;
            }
            RequestAction::ListPlayers => {
                return self.list_players(player_id);
            }
            RequestAction::ChatMessage { message } => {
                return self.handle_chat_message(player_id, message);
            }
            RequestAction::ListRooms => {
                return self.list_rooms();
            }
            RequestAction::NewRoom { room_name } => {
                return self.create_new_room(Some(player_id), room_name);
            }
            RequestAction::JoinRoom { room_name } => {
                return self.join_room(player_id, &room_name);
            }
            RequestAction::LeaveRoom => {
                return self.leave_room(player_id);
            }
            RequestAction::Connect { .. } => {
                return ResponseCode::BadRequest {
                    error_msg: "Already connected".to_owned(),
                };
            }
            RequestAction::None => {
                return ResponseCode::BadRequest {
                    error_msg: format!("Invalid request: {:?}", action),
                };
            }
        }
    }

    pub fn is_player_in_game(&self, player_id: PlayerID) -> bool {
        let player: Option<&Player> = self.players.get(&player_id);
        player.is_some() && player.unwrap().game_info.is_some()
    }

    pub fn is_unique_player_name(&self, name: &str) -> bool {
        for ref player in self.players.values() {
            if player.name == name {
                return false;
            }
        }
        return true;
    }

    // Request_ack contains the last processed sequence number. If one arrives older (less than)
    // than the last processed, it must be rejected.
    // FIXME Does not handle wrapped sequence number case yet.
    pub fn is_previously_processed_packet(&mut self, player_id: PlayerID, sequence: u64) -> bool {
        let player: &Player = self.get_player(player_id);
        if let Some(request_ack) = player.request_ack {
            if sequence <= request_ack {
                return true;
            }
        }
        false
    }

    pub fn get_player_id_by_cookie(&self, cookie: &str) -> Option<PlayerID> {
        match self.player_map.get(cookie) {
            Some(player_id) => Some(*player_id),
            None => None,
        }
    }

    /// Returns true if the packet already exists in the queue, otherwise it will return false, and
    /// will be added in sequence_number order.
    pub fn add_packet_to_queue(&mut self, player_id: PlayerID, packet: Packet) -> bool {
        // Unwrap should be safe since a player ID was already found.
        let network: &mut NetworkManager = self.network_map.get_mut(&player_id).unwrap();
        let already_exists = network.rx_packets.buffer_item(packet);
        already_exists
    }

    /// Checks to see if the incoming packet is immediately processable
    pub fn can_process_packet(&mut self, player_id: PlayerID, sequence_number: u64) -> bool {
        let player: &mut Player = self.get_player_mut(player_id);
        if let Some(ack) = player.request_ack {
            trace!("[CAN PROCESS?] Ack: {} Sqn: {}", ack, sequence_number);
            ack + 1 == sequence_number
        } else {
            // request_ack has not been set yet, likely first packet
            player.request_ack = Some(0);
            true
        }
    }

    /// Processes a player's request action for all non-logged in requests. If necessary, a response is buffered
    /// for later transmission
    pub fn process_player_request_action(
        &mut self,
        player_id: PlayerID,
        action: RequestAction,
    ) -> Result<Option<Packet>, Box<dyn Error>> {
        match action {
            RequestAction::Connect { .. } => unreachable!(),
            _ => {
                if let Some(response) = self.prepare_response(player_id, action.clone()) {
                    // Buffer all responses to the client for [re-]transmission
                    let network: Option<&mut NetworkManager> = self.network_map.get_mut(&player_id);
                    if let Some(player_net) = network {
                        trace!("[A Response to Client Request added to TX Buffer]{:?}", response);
                        player_net.tx_packets.buffer_item(response.clone());
                    }
                    Ok(Some(response))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Determine how many contiguous packets are processable and process their requests for the given player.
    pub fn process_queued_rx_packets(&mut self, player_id: PlayerID) {
        // If we can, start popping off the RX queue and handle contiguous packets immediately
        let mut dequeue_count = 0;

        // Get the last packet we've sent to this player
        let player_processed_seq_num = self.get_player(player_id).request_ack;
        let mut latest_processed_seq_num;

        if let Some(seq_num) = player_processed_seq_num {
            latest_processed_seq_num = seq_num;
        } else {
            // if request_ack is None, we shouldn't have processed anything yet
            latest_processed_seq_num = 0;
        }

        // Collect the next batch of received packets we can process.
        let rx_queue_count;
        let mut processable_packets: Vec<Packet> = vec![];
        {
            let network: Option<&mut NetworkManager> = self.network_map.get_mut(&player_id);
            if let Some(player_net) = network {
                rx_queue_count = player_net
                    .rx_packets
                    .get_contiguous_packets_count(latest_processed_seq_num + 1);
                // ameen: can I use take().filter().collect()?
                while dequeue_count < rx_queue_count {
                    let packet = player_net.rx_packets.as_queue_type_mut().pop_front().unwrap();

                    // It is possible that a previously buffered packet (due to out-of-order) was resent by the client,
                    // and processed immediately upon receipt. We need to skip these.
                    if packet.sequence_number() > latest_processed_seq_num {
                        processable_packets.push(packet);
                    }

                    dequeue_count += 1;
                }
            }
        }

        for packet in processable_packets {
            trace!("[Processing Client Request from RX Buffer]: {:?}", packet);
            match packet {
                Packet::Request {
                    sequence,
                    response_ack: _,
                    cookie: _,
                    action,
                } => {
                    latest_processed_seq_num += 1;
                    assert!(sequence == latest_processed_seq_num);
                    let _response_packet = self.process_player_request_action(player_id, action);
                }
                _ => panic!("Development bug: Non-response packet found in client buffered RX queue"),
            }
        }
    }

    pub fn process_player_buffered_packets(&mut self, players_to_update: &Vec<PlayerID>) {
        for player_id in players_to_update {
            self.process_queued_rx_packets(*player_id);
        }
    }

    pub fn process_buffered_packets_in_lobby(&mut self) {
        let mut players_to_update: Vec<PlayerID> = vec![];

        if self.players.len() == 0 {
            return;
        }

        // Skip players if they're in a game
        for player in self.players.values() {
            if player.game_info.is_some() {
                continue;
            }
            players_to_update.push(player.player_id);
        }

        if players_to_update.len() != 0 {
            self.process_player_buffered_packets(&players_to_update);
        }
    }

    pub fn collect_expired_tx_packets(&mut self) -> Vec<(Packet, SocketAddr)> {
        if self.players.len() == 0 {
            return vec![];
        }

        let mut players_to_update: Vec<PlayerID> = vec![];

        for player in self.players.values() {
            players_to_update.push(player.player_id);
        }

        let mut expired_responses = vec![];
        if players_to_update.len() != 0 {
            for player_id in players_to_update {
                // If any processed packets result in responses, prepare them below for transmission
                let player_addr: SocketAddr = self.get_player_mut(player_id).addr;
                let ack = self.get_player_mut(player_id).request_ack;

                let player_network: Option<&mut NetworkManager> = self.network_map.get_mut(&player_id);
                if let Some(player_net) = player_network {
                    if player_net.tx_packets.len() == 0 {
                        continue;
                    }

                    let indices = player_net.tx_packets.get_retransmit_indices();
                    trace!(
                        "[Sending expired responses to client from TX Buffer]: {:?} Len: {}",
                        player_id,
                        indices.len()
                    );
                    let retransmissions = player_net.retransmit_expired_tx_packets(player_addr, ack, &indices);
                    expired_responses.extend_from_slice(retransmissions.as_slice());
                } else {
                    error!("I haven't found a NetworkManager for Player: {}", player_id);
                    continue;
                }
            }
        }

        return expired_responses;
    }

    pub fn process_buffered_packets_in_rooms(&mut self) {
        let mut players_to_update: Vec<PlayerID> = vec![];

        if self.rooms.len() == 0 {
            return;
        }

        for room in self.rooms.values() {
            if room.player_ids.len() == 0 {
                continue;
            }

            for &player_id in &room.player_ids {
                let opt_player: Option<&Player> = self.players.get(&player_id);
                if opt_player.is_none() {
                    continue;
                }

                let player: &Player = opt_player.unwrap();
                if player.game_info.is_none() {
                    continue;
                }

                players_to_update.push(player_id);
            }
        }

        if players_to_update.len() != 0 {
            self.process_player_buffered_packets(&players_to_update);
        }
    }

    /// Clear out the transmission queue of any packets the client has acknowledged
    pub fn clear_transmission_queue_on_ack(&mut self, player_id: PlayerID, response_ack: Option<u64>) {
        if let Some(response_ack) = response_ack {
            if let Some(ref mut player_network) = self.network_map.get_mut(&player_id) {
                let mut removal_count = 0;
                for resp_pkt in player_network.tx_packets.as_queue_type_mut().iter() {
                    if response_ack > 0 && (resp_pkt.sequence_number() <= response_ack - 1) {
                        removal_count += 1;
                    } else {
                        break;
                    }
                    // else {
                    // TODO handle wrapped case & unit tests
                    // }
                }

                if removal_count != 0 {
                    player_network.tx_pop_front_with_count(removal_count);
                }
            }
        }
    }

    /// Inspect the packet's contents for acceptance, or reject it.
    /// `Response`/`Update` packets are invalid in this context
    /// Acceptance criteria for `Request`
    ///  1. Cookie must be present and valid
    ///  2. Ignore KeepAlive
    ///  3. Client should notified if version requires updating
    ///  4. Ignore if already received or processed
    /// Always returns either Ok(Some(Packet::Response{...})), Ok(None), or error.
    pub fn decode_packet(&mut self, addr: SocketAddr, packet: Packet) -> Result<Option<Packet>, Box<dyn Error>> {
        match packet.clone() {
            Packet::Response { .. } | Packet::Update { .. } | Packet::Status { .. } => {
                return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "invalid packet type")));
            }
            Packet::Request {
                sequence,
                response_ack,
                cookie,
                action,
            } => {
                match action {
                    RequestAction::Connect { .. } => (),
                    RequestAction::KeepAlive { latest_response_ack: _ } => (),
                    _ => {
                        if cookie == None {
                            return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "no cookie")));
                        } else {
                            trace!(
                                "[Request] cookie: {:?} sequence: {} resp_ack: {:?} event: {:?}",
                                cookie,
                                sequence,
                                response_ack,
                                action
                            );
                        }
                    }
                }
                // handle connect (create user, and save cookie)
                if let RequestAction::Connect { name, client_version } = action {
                    if validate_client_version(client_version) {
                        let response = self.handle_new_connection(name, addr);
                        return Ok(Some(response));
                    } else {
                        return Err(Box::new(io::Error::new(
                            ErrorKind::Other,
                            "client out of date -- please upgrade",
                        )));
                    };
                } else {
                    // look up player by cookie
                    let cookie = match cookie {
                        Some(cookie) => cookie,
                        None => {
                            return Err(Box::new(io::Error::new(
                                ErrorKind::InvalidData,
                                "cookie required for non-connect actions",
                            )));
                        }
                    };
                    let player_id = match self.get_player_id_by_cookie(cookie.as_str()) {
                        Some(player_id) => player_id,
                        None => {
                            return Err(Box::new(io::Error::new(ErrorKind::PermissionDenied, "invalid cookie")));
                        }
                    };

                    let mut player: &mut Player = self.get_player_mut(player_id);
                    player.last_received = time::Instant::now(); // reset time of last received packet from player
                    match action.clone() {
                        RequestAction::KeepAlive { latest_response_ack } => {
                            // If the client does not send new requests, the Server will never get a reply for
                            // the set of responses it may have sent. This will result in the transmission queue contents
                            // flooding the Client on retranmission.
                            self.clear_transmission_queue_on_ack(player_id, Some(latest_response_ack));
                            return Ok(None);
                        }
                        _ => (),
                    }

                    // For the non-KeepAlive case
                    self.clear_transmission_queue_on_ack(player_id, response_ack);

                    // Check to see if it can be processed right away, otherwise buffer it for later consumption.
                    // Not sure if I like this name but it'll do for now.
                    if self.can_process_packet(player_id, sequence) {
                        trace!("[PROCESS IMMEDIATE]");
                        return self.process_player_request_action(player_id, action);
                    }

                    // Packet may be resent by client but has since been processed.
                    if self.is_previously_processed_packet(player_id, sequence) {
                        trace!("\t [ALREADY PROCESSED]");
                        return Ok(None);
                    }

                    // Returns true if the packet already exists in the queue
                    if self.add_packet_to_queue(player_id, packet) {
                        trace!("\t [ALREADY QUEUED]");
                        return Ok(None);
                    }

                    // In the event we buffered it, we do not send a response
                    trace!("\t [BUFFERED]");
                    Ok(None)
                }
            }
            Packet::UpdateReply {
                cookie,
                last_chat_seq,
                last_game_update_seq: _,
                last_gen: _,
                pong: _,
            } => {
                let opt_player_id = self.get_player_id_by_cookie(cookie.as_str());

                if opt_player_id.is_none() {
                    return Err(Box::new(io::Error::new(ErrorKind::PermissionDenied, "invalid cookie")));
                }

                let player_id = opt_player_id.unwrap();
                let opt_player = self.players.get_mut(&player_id);

                if opt_player.is_none() {
                    return Err(Box::new(io::Error::new(ErrorKind::NotFound, "player not found")));
                }

                let player: &mut Player = opt_player.unwrap();

                if player.game_info.is_some() {
                    player.update_chat_seq_num(last_chat_seq);
                }

                player.latency_filter.update();

                Ok(None)
            }
            Packet::GetStatus { ping } => Ok(Some(self.get_status(ping.nonce))),
        }
    }

    fn get_status(&self, nonce: u64) -> Packet {
        Packet::Status {
            pong:           PingPong { nonce },
            player_count:   self.player_map.len() as u64,
            room_count:     self.room_map.len() as u64,
            server_name:    "Leto II".to_owned(),
            server_version: VERSION.to_owned(),
        }
    }

    pub fn prepare_response(&mut self, player_id: PlayerID, action: RequestAction) -> Option<Packet> {
        let response_code = self.process_request_action(player_id, action.clone());

        let (sequence, request_ack);

        match action {
            // Filtered away at the decoding packet layer
            RequestAction::KeepAlive { latest_response_ack: _ } => unreachable!(),
            // Prepare a response for all other requests
            _ => {
                let opt_player: Option<&mut Player> = self.players.get_mut(&player_id);

                match opt_player {
                    Some(player) => {
                        sequence = player.increment_response_seq_num();
                        if let Some(ack) = player.request_ack {
                            player.request_ack = Some(ack + 1);
                            request_ack = player.request_ack;
                        } else {
                            panic!("Player's request ack has never been set. It should have been set after the first packet!");
                        }
                    }
                    None => {
                        // This happens with Disconnect packets -- player was deleted at top of this
                        // function.
                        return None;
                    }
                }
            }
        }

        Some(Packet::Response {
            sequence:    sequence,
            request_ack: request_ack,
            code:        response_code,
        })
    }

    pub fn handle_new_connection(&mut self, name: String, addr: SocketAddr) -> Packet {
        if self.is_unique_player_name(&name) {
            let player = self.add_new_player(name, addr.clone());
            let cookie = player.cookie.clone();

            // Sequence is assumed to start at 0 for all new connections
            let response = Packet::Response {
                sequence:    0,
                request_ack: Some(0), // Should start at seq_num 0 unless client's network state was not properly reset
                code:        ResponseCode::LoggedIn {
                    cookie,
                    server_version: VERSION.to_owned(),
                },
            };
            return response;
        } else {
            // not a unique name
            let response = Packet::Response {
                sequence:    0,
                request_ack: None,
                code:        ResponseCode::Unauthorized {
                    error_msg: "not a unique name".to_owned(),
                },
            };
            return response;
        }
    }

    // Right now we'll be constructing all client Update packets for _every_ room.
    pub fn construct_client_updates(&mut self) -> Vec<(SocketAddr, Packet)> {
        let mut client_updates: Vec<(SocketAddr, Packet)> = vec![];

        if self.rooms.len() == 0 {
            return vec![];
        }

        // For each room, determine if each player has unread messages based on chat_msg_seq_num
        // TODO: POOR PERFORMANCE BOUNTY
        for room in self.rooms.values() {
            if room.messages.is_empty() || room.player_ids.len() == 0 {
                continue;
            }

            for &player_id in &room.player_ids {
                let opt_player = self.players.get(&player_id);
                if opt_player.is_none() {
                    continue;
                }

                let player: &Player = opt_player.unwrap();
                if player.game_info.is_none() {
                    continue;
                }

                let mut unsent_messages = vec![];
                if let Some(new_messages) = self.collect_unacknowledged_messages(&room, player) {
                    unsent_messages = new_messages.to_vec();
                }

                let messages_available = unsent_messages.len() != 0;
                // XXX Requires implementation
                let game_updates_available = false;
                let universe_updates_available = false;

                let update_packet = Packet::Update {
                    chats:           unsent_messages,
                    game_updates:    vec![],
                    universe_update: UniUpdateType::NoChange,
                    ping:            PingPong::ping(),
                };

                if messages_available || game_updates_available || universe_updates_available {
                    client_updates.push((player.addr.clone(), update_packet));
                }
            }
        }

        return client_updates;
    }

    /// Creates a vector of messages that the provided Player has not yet acknowledged.
    /// Exits early if the player is already caught up.
    pub fn collect_unacknowledged_messages(&self, room: &Room, player: &Player) -> Option<Vec<BroadcastChatMessage>> {
        // Only send what a player has not yet seen
        let raw_unsent_messages: VecDeque<ServerChatMessage>;
        match player.get_confirmed_chat_seq_num() {
            Some(chat_msg_seq_num) => {
                let opt_newest_msg = room.get_newest_msg();
                if opt_newest_msg.is_none() {
                    return None;
                }

                let newest_msg = opt_newest_msg.unwrap();

                if chat_msg_seq_num == newest_msg.seq_num {
                    // Player is caught up
                    return None;
                } else if chat_msg_seq_num > newest_msg.seq_num {
                    error!(
                        "Misbehaving client {:?};\nClient says it has more messages than we sent!",
                        player
                    );
                    return None;
                } else {
                    let amount_to_consume = room.get_message_skip_count(chat_msg_seq_num);

                    // Cast to usize is safe because our message containers are limited by MAX_NUM_CHAT_MESSAGES
                    raw_unsent_messages = room.messages.iter().skip(amount_to_consume as usize).cloned().collect();
                }
            }
            None => {
                // Smithers, unleash the hounds!
                raw_unsent_messages = room.messages.clone();
            }
        };

        if raw_unsent_messages.len() == 0 {
            return None;
        }

        let unsent_messages: Vec<BroadcastChatMessage> = raw_unsent_messages
            .iter()
            .map(|msg| BroadcastChatMessage::new(msg.seq_num, msg.player_name.clone(), msg.message.clone()))
            .collect();

        return Some(unsent_messages);
    }

    pub fn expire_old_messages_in_all_rooms(&mut self, current_timestamp: time::Instant) {
        if self.rooms.len() != 0 {
            for room in self.rooms.values_mut() {
                if room.has_players() && !room.messages.is_empty() {
                    room.messages.retain(|ref m| {
                        current_timestamp - m.timestamp < Duration::from_secs(MAX_AGE_CHAT_MESSAGES as u64)
                    });
                }
            }
        }
    }

    pub fn add_new_player(&mut self, name: String, addr: SocketAddr) -> &mut Player {
        let cookie = new_cookie();
        let player_id = PlayerID(new_uuid());
        let player = Player {
            player_id:      player_id.clone(),
            cookie:         cookie.clone(),
            addr:           addr,
            name:           name,
            request_ack:    None,
            next_resp_seq:  0,
            game_info:      None,
            last_received:  Instant::now(),
            latency_filter: LatencyFilter::new(),
        };

        // save player into players hash map, and save player ID into hash map using cookie
        self.player_map.insert(cookie, player_id);
        self.players.insert(player_id, player);
        self.network_map.insert(player_id, NetworkManager::new());

        let player = self.get_player_mut(player_id);

        // We expect that the Server proceed with `1` after the connection has been established
        player.increment_response_seq_num();
        player
    }

    pub fn remove_timed_out_clients(&mut self) {
        let mut timed_out_players: Vec<PlayerID> = vec![];

        for (p_id, p) in self.players.iter() {
            if has_connection_timed_out(p.last_received) {
                info!("Player(cookie={:?}) has timed out", p.cookie);
                timed_out_players.push(*p_id);
            }
        }

        for player_id in timed_out_players {
            self.handle_disconnect(player_id);
        }
    }

    /// Creates a new struct representing the global state of this server. Initially, there is one
    /// room -- "general".
    pub fn new() -> Self {
        let mut server_state = ServerState {
            tick:        0,
            players:     HashMap::<PlayerID, Player>::new(),
            rooms:       HashMap::<RoomID, Room>::new(),
            player_map:  HashMap::<String, PlayerID>::new(),
            room_map:    HashMap::<String, RoomID>::new(),
            network_map: HashMap::<PlayerID, NetworkManager>::new(),
        };
        server_state.new_room("general".to_owned());
        server_state
    }

    fn process_packet(&mut self, packet_tuple: (Packet, SocketAddr)) -> Vec<(Packet, SocketAddr)> {
        let (packet, addr) = packet_tuple;

        debug!("{:?}", packet);

        // Decode incoming and send a Response to the Requester
        let decode_result = self.decode_packet(addr, packet.clone());
        if let Ok(opt_response_packet) = decode_result {
            if let Some(response_packet) = opt_response_packet {
                let response = (response_packet, addr.clone());
                return vec![response];
            }
        } else {
            let err = decode_result.unwrap_err();
            error!("Decoding packet failed, from {:?}: {:?}", addr, err);
        }

        vec![]
    }

    fn send_heartbeats(&mut self) -> Vec<(Packet, SocketAddr)> {
        let mut heartbeats = vec![];
        for player in self.players.values() {
            let keep_alive = Packet::Response {
                sequence:    0,
                request_ack: None,
                code:        ResponseCode::KeepAlive,
            };
            heartbeats.push((keep_alive, player.addr));
        }
        return heartbeats;
    }

    fn maintain_network_state(&mut self) -> Vec<(Packet, SocketAddr)> {
        // Process players in rooms
        self.process_buffered_packets_in_rooms();

        // Process players in lobby
        self.process_buffered_packets_in_lobby();

        self.collect_expired_tx_packets()
    }

    fn garbage_collection(&mut self) -> Vec<(SocketAddr, Packet)> {
        self.expire_old_messages_in_all_rooms(time::Instant::now());
        let update_packets_vec = self.construct_client_updates();

        self.remove_timed_out_clients();
        self.tick = 1usize.wrapping_add(self.tick);
        return update_packets_vec;
    }
}

//////////////// Event Handling /////////////////
#[allow(unused)]
enum Event {
    TickEvent,
    Request((SocketAddr, Option<Packet>)),
    NetworkTickEvent,
    HeartBeat,
    //    Notify((SocketAddr, Option<Packet>)),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{:5}] - {}",
                Local::now().format("%a %Y-%m-%d %H:%M:%S%.6f"),
                record.level(),
                record.args(),
            )
        })
        .filter(None, LevelFilter::Trace)
        .filter(Some("futures"), LevelFilter::Off)
        .filter(Some("tokio_core"), LevelFilter::Off)
        .filter(Some("tokio_reactor"), LevelFilter::Off)
        .init();

    let matches = App::new("server")
        .about("game server for Conwayste")
        .arg(
            Arg::with_name("address")
                .short("l")
                .long("listen")
                .help(&format!(
                    "address to listen for connections on [default {}]",
                    DEFAULT_HOST
                ))
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help(&format!("port to listen for connections on [default {}]", DEFAULT_PORT))
                .takes_value(true),
        )
        .get_matches();

    let opt_host = matches.value_of("address");
    let opt_port = matches.value_of("port").map(|port_str| {
        port_str.parse::<u16>().unwrap_or_else(|e| {
            error!("Error while attempting to parse {:?} as port number: {:?}", port_str, e);
            exit(1);
        })
    });

    let udp = bind(opt_host, opt_port).await.unwrap_or_else(|e| {
        error!("Error while trying to bind UDP socket: {:?}", e);
        exit(1);
    });

    trace!("Listening for connections on {:?}...", udp.local_addr()?);

    let (mut udp_sink, udp_stream) = UdpFramed::new(udp, NetwaystePacketCodec).split();
    let mut udp_stream = udp_stream.fuse();

    let mut server_state = ServerState::new();

    let mut tick_interval = TT::interval(Duration::from_millis(TICK_INTERVAL_IN_MS)).fuse();
    let mut network_interval = TT::interval(Duration::from_millis(NETWORK_INTERVAL_IN_MS)).fuse();
    let mut heartbeat_interval = TT::interval(Duration::from_millis(HEARTBEAT_INTERVAL_IN_MS)).fuse();

    loop {
        select! {
            (_) = tick_interval.select_next_some() => {
                let update_packets = server_state.garbage_collection();
                for (addr, packet) in update_packets {
                    udp_sink.send((packet, addr)).await?;
                }
            },
            (_) = network_interval.select_next_some() => {
                let retranmissions = server_state.maintain_network_state();
                for packet_addr_tuple in retranmissions {
                    udp_sink.send(packet_addr_tuple).await?;
                }
            },
            (_) = heartbeat_interval.select_next_some() => {
                let heartbeats = server_state.send_heartbeats();
                for packet_addr_tuple in heartbeats {
                    udp_sink.send(packet_addr_tuple).await?;
                }
            },
            (addr_packet_result) = udp_stream.select_next_some() => {
                if let Ok(addr_packet_tuple) = addr_packet_result {
                    let responses = server_state.process_packet(addr_packet_tuple);
                    for response in responses {
                        udp_sink.send(response).await?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod netwayste_server_tests {
    use super::*;
    use ::proptest::strategy::*;
    use netwayste::net::NetAttempt;

    fn fake_socket_addr() -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5678)
    }

    #[test]
    fn list_players_player_shows_up_in_player_list() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));

        let (player_id, player_name) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.name.clone())
        };
        // make the player join the room
        {
            server.join_room(player_id, room_name);
        }
        let resp_code: ResponseCode = server.list_players(player_id);
        match resp_code {
            ResponseCode::PlayerList { players } => {
                assert_eq!(players.len(), 1);
                assert_eq!(*players.first().unwrap(), player_name);
            }
            resp_code @ _ => panic!("Unexpected response code: {:?}", resp_code),
        }
    }

    #[test]
    fn has_chatted_player_did_not_chat_on_join() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));
        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());
            p.player_id
        };
        // make the player join the room
        {
            server.join_room(player_id, room_name);
        }
        let player = server.get_player(player_id);
        assert_eq!(player.has_chatted(), false);
    }

    #[test]
    fn get_confirmed_chat_seq_num_server_tracks_players_chat_updates() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));

        let (player_id, player_cookie) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.cookie.clone())
        };
        // make the player join the room
        {
            server.join_room(player_id, room_name);
        }

        // A chat-less player now has something to to say
        server
            .decode_packet(
                fake_socket_addr(),
                Packet::UpdateReply {
                    cookie:               player_cookie.clone(),
                    last_chat_seq:        Some(1),
                    last_game_update_seq: None,
                    last_gen:             None,
                    pong:                 PingPong::pong(0),
                },
            )
            .unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }

        // Older messages are ignored
        server
            .decode_packet(
                fake_socket_addr(),
                Packet::UpdateReply {
                    cookie:               player_cookie.clone(),
                    last_chat_seq:        Some(0),
                    last_game_update_seq: None,
                    last_gen:             None,
                    pong:                 PingPong::pong(0),
                },
            )
            .unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }

        // So are absent messages
        server
            .decode_packet(
                fake_socket_addr(),
                Packet::UpdateReply {
                    cookie:               player_cookie,
                    last_chat_seq:        None,
                    last_game_update_seq: None,
                    last_gen:             None,
                    pong:                 PingPong::pong(0),
                },
            )
            .unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }
    }

    #[test]
    fn get_message_skip_count_player_acked_messages_are_not_included_in_skip_count() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let (player_id, _) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.cookie.clone())
        };
        // make the player join the room
        // Give it a single message
        {
            server.join_room(player_id, room_name);
            server.handle_chat_message(player_id, "ChatMessage".to_owned());
        }

        {
            let room: &Room = server.get_room(player_id).unwrap();
            // The check below does not affect any player acknowledgement as we are not
            // involving the player yet. This is a simple test to ensure that if a chat
            // message decoded from a would-be player was less than the latest chat message,
            // we handle it properly by not skipping any.
            assert_eq!(room.get_message_skip_count(0), 0);
        }

        let number_of_messages = 6;
        for _ in 1..number_of_messages {
            server.handle_chat_message(player_id, "ChatMessage".to_owned());
        }

        {
            //let player = server.get_player_mut(player_id);
            let player = server.get_player_mut(player_id);
            // player has not acknowledged any yet
            #[should_panic]
            assert_eq!(player.get_confirmed_chat_seq_num(), None);
        }

        // player acknowledged four of the six
        let acked_message_count = {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(4));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.get_message_skip_count(acked_message_count), acked_message_count);
        }

        // player acknowledged all six
        let acked_message_count = {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(6));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.get_message_skip_count(acked_message_count), acked_message_count);
        }
    }

    #[test]
    // Send fifteen messages, but only leave nine unacknowledged, while wrapping on the sequence number
    fn get_message_skip_count_player_acked_messages_are_not_included_in_skip_count_wrapped_case() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name);
        }

        // Picking a value slightly less than max of u64
        let start_seq_num = u64::max_value() - 6;
        // First pass, add messages with sequence numbers through the max of u64
        for seq_num in start_seq_num..u64::max_value() {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(
                player_id,
                String::from("some name"),
                String::from("some msg"),
                seq_num,
            ));
        }
        // Second pass, from wrap-point, `0`, eight times
        for seq_num in 0..8 {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(
                player_id,
                String::from("some name"),
                String::from("some msg"),
                seq_num,
            ));
        }

        let acked_message_count = {
            // Ack up until 0xFFFFFFFFFFFFFFFD
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(start_seq_num + 4));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            // Fifteen total messages sent.
            // 2 unacked which are less than u64::max_value()
            // 8 unacked which are after the numerical wrap
            let unacked_count = 15 - (8 + 2);
            assert_eq!(room.get_message_skip_count(acked_message_count), unacked_count);
        }
    }

    #[test]
    fn collect_unacknowledged_messages_a_rooms_unacknowledged_chat_messages_are_collected_for_their_player() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name);
        }

        {
            // Room has no messages, None to send to player
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }

        {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(
                player_id,
                String::from("some name"),
                String::from("some msg"),
                1,
            ));
        }
        {
            // Room has a message, player has yet to ack it
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages.is_some(), true);
            assert_eq!(messages.unwrap().len(), 1);
        }

        {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(1));
        }
        {
            // Room has a message, player acked, None
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }
    }

    #[test]
    fn collect_unacknowledged_messages_an_active_room_which_expired_all_messages_returns_none() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name);
        }

        {
            // Add a message to the room and then age it so it will expire
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(
                player_id,
                String::from("some name"),
                String::from("some msg"),
                1,
            ));

            let message: &mut ServerChatMessage = room.messages.get_mut(0).unwrap();
            let travel_to_the_past = Instant::now().checked_sub(Duration::from_secs(MAX_AGE_CHAT_MESSAGES as u64));
            if travel_to_the_past.is_none() {
                warn!("skipping rest of test; cannot travel to the past :(");
                return;
            }
            message.timestamp = travel_to_the_past.unwrap();
        }
        {
            // Sanity check to ensure player gets the chat message if left unacknowledged
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages.is_some(), true);
            assert_eq!(messages.unwrap().len(), 1);
        }
        {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(1));
        }

        {
            // Server drains expired messages for the room
            server.expire_old_messages_in_all_rooms(time::Instant::now());
        }
        {
            // A room that has no messages, but has player(s) who have acknowledged past messages
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }
    }

    #[test]
    fn handle_chat_message_player_not_in_game() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some name".to_owned(), fake_socket_addr());

            p.player_id
        };

        let response = server.handle_chat_message(player_id, "test msg".to_owned());
        assert_eq!(
            response,
            ResponseCode::BadRequest {
                error_msg: format!("Player {} has not joined a game.", player_id),
            }
        );
    }

    #[test]
    fn handle_chat_message_player_in_game_one_message() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_string(), fake_socket_addr());

            p.player_id
        };
        server.join_room(player_id, room_name);

        let response = server.handle_chat_message(player_id, "test msg".to_owned());
        assert_eq!(response, ResponseCode::OK);
        let room: &Room = server.get_room(player_id).unwrap();
        assert_eq!(room.messages.len(), 1);
        assert_eq!(room.latest_seq_num, 1);
        assert_eq!(room.get_newest_msg(), room.get_oldest_msg());
    }

    #[test]
    fn handle_chat_message_player_in_game_many_messages() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        server.join_room(player_id, room_name);

        let response = server.handle_chat_message(player_id, "test msg first".to_owned());
        assert_eq!(response, ResponseCode::OK);
        let response = server.handle_chat_message(player_id, "test msg second".to_owned());
        assert_eq!(response, ResponseCode::OK);

        let room: &Room = server.get_room(player_id).unwrap();
        assert_eq!(room.messages.len(), 2);
        assert_eq!(room.latest_seq_num, 2);
    }

    #[test]
    fn create_new_room_good_case() {
        {
            let mut server = ServerState::new();
            let room_name = "some name".to_owned();

            assert_eq!(server.create_new_room(None, room_name), ResponseCode::OK);
        }
        // Room name length is within bounds
        {
            let mut server = ServerState::new();
            let room_name = "0123456789ABCDEF".to_owned();

            assert_eq!(server.create_new_room(None, room_name), ResponseCode::OK);
        }
    }

    #[test]
    fn create_new_room_name_is_too_long() {
        let mut server = ServerState::new();
        let room_name = "0123456789ABCDEF_#".to_owned();

        assert_eq!(
            server.create_new_room(None, room_name),
            ResponseCode::BadRequest {
                error_msg: "room name too long; max 16 characters".to_owned(),
            }
        );
    }

    #[test]
    fn create_new_room_name_taken() {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);
        assert_eq!(
            server.create_new_room(None, room_name),
            ResponseCode::BadRequest {
                error_msg: "room name already in use".to_owned(),
            }
        );
    }

    #[test]
    fn create_new_room_player_already_in_room() {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        let other_room_name = "another room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        server.join_room(player_id, &room_name);

        assert_eq!(
            server.create_new_room(Some(player_id), other_room_name),
            ResponseCode::BadRequest {
                error_msg: "cannot create room because in-game".to_owned(),
            }
        );
    }

    #[test]
    fn create_new_room_join_room_good_case() {
        let mut server = ServerState::new();
        let room_name = "some room";
        assert_eq!(server.create_new_room(None, room_name.to_owned()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(
            server.join_room(player_id, room_name),
            ResponseCode::JoinedRoom {
                room_name: "some room".to_owned(),
            }
        );
    }

    #[test]
    fn join_room_player_already_in_room() {
        let mut server = ServerState::new();
        let room_name = "some room";
        assert_eq!(server.create_new_room(None, room_name.to_owned()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(
            server.join_room(player_id, room_name),
            ResponseCode::JoinedRoom {
                room_name: "some room".to_owned(),
            }
        );
        assert_eq!(
            server.join_room(player_id, room_name),
            ResponseCode::BadRequest {
                error_msg: "cannot join game because in-game".to_owned(),
            }
        );
    }

    #[test]
    fn join_room_room_does_not_exist() {
        let mut server = ServerState::new();

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(
            server.join_room(player_id, "some room"),
            ResponseCode::BadRequest {
                error_msg: "no room named \"some room\"".to_owned(),
            }
        );
    }

    #[test]
    fn leave_room_good_case() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        server.join_room(player_id, room_name);

        assert_eq!(server.leave_room(player_id), ResponseCode::LeaveRoom);
    }

    #[test]
    fn leave_room_player_not_in_room() {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };

        assert_eq!(
            server.leave_room(player_id),
            ResponseCode::BadRequest {
                error_msg: "cannot leave game because in lobby".to_owned(),
            }
        );
    }

    #[test]
    fn leave_room_unregistered_player_id() {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        let rand_player_id = PlayerID(0x2457); //RUST
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        assert_eq!(
            server.leave_room(rand_player_id),
            ResponseCode::BadRequest {
                error_msg: "cannot leave game because in lobby".to_owned(),
            }
        );
    }

    #[test]
    fn add_new_player_player_added_with_initial_sequence_number() {
        let mut server = ServerState::new();
        let name = "some player".to_owned();

        let p: &mut Player = server.add_new_player(name.clone(), fake_socket_addr());
        assert_eq!(p.name, name);
    }

    #[test]
    fn is_unique_player_name_yes_and_no_case() {
        let mut server = ServerState::new();
        let name = "some player".to_owned();
        assert_eq!(server.is_unique_player_name("some player"), true);

        {
            server.add_new_player(name.clone(), fake_socket_addr());
        }
        assert_eq!(server.is_unique_player_name("some player"), false);
    }

    #[test]
    fn expire_old_messages_in_all_rooms_room_is_empty() {
        let mut server = ServerState::new();
        let room_name = "some room";

        server.create_new_room(None, room_name.to_owned().clone());
        server.expire_old_messages_in_all_rooms(time::Instant::now());

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_one_room_good_case() {
        let mut server = ServerState::new();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, "general");

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id, "It is free!".to_owned());
        server.handle_chat_message(player_id, "What's not to love?".to_owned());

        let message_count = {
            let room: &Room = server.get_room(player_id).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count, 4);

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms(time::Instant::now());

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 4);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_several_rooms_good_case() {
        let mut server = ServerState::new();
        let room_name = "some room";
        let room_name2 = "some room2";

        let room_id = server.new_room(room_name.to_owned());
        let room_id2 = server.new_room(room_name2.to_owned());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name);
        server.join_room(player_id2, room_name2);

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id2, "It is free!".to_owned());
        server.handle_chat_message(player_id2, "What's not to love?".to_owned());

        let message_count = {
            let room: &Room = server.get_room(player_id).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count, 2);
        let message_count2 = {
            let room: &Room = server.get_room(player_id2).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count2, 2);

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms(time::Instant::now());

        assert_eq!(server.rooms[&room_id].messages.len(), 2);
        assert_eq!(server.rooms[&room_id2].messages.len(), 2);
    }

    #[test]
    fn expire_old_messages_in_all_rooms_one_room_old_messages_are_wiped() {
        let mut server = ServerState::new();
        let room_name = "some room";

        server.create_new_room(None, room_name.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name);

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id, "It is free!".to_owned());
        server.handle_chat_message(player_id, "What's not to love?".to_owned());

        let current_timestamp = Instant::now();
        let travel_to_the_past = current_timestamp.checked_sub(Duration::from_secs((MAX_AGE_CHAT_MESSAGES + 1) as u64));
        if travel_to_the_past.is_none() {
            warn!("skipping rest of test; cannot travel to the past :(");
            return;
        }
        let travel_to_the_past = travel_to_the_past.unwrap();
        for ref mut room in server.rooms.values_mut() {
            println!("Room: {:?}", room.name);
            for m in room.messages.iter_mut() {
                println!(
                    "{:?}, {:?},       {:?}",
                    m.timestamp,
                    travel_to_the_past,
                    m.timestamp - travel_to_the_past
                );
                m.timestamp = travel_to_the_past;
            }
        }

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms(time::Instant::now());

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_several_rooms_old_messages_are_wiped() {
        let mut server = ServerState::new();
        let room_name = "some room";
        let room_name2 = "some room 2";

        server.create_new_room(None, room_name.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        server.create_new_room(None, room_name2.to_owned().clone());
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name);
        server.join_room(player_id2, room_name);

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id2, "It is free!".to_owned());
        server.handle_chat_message(player_id2, "What's not to love?".to_owned());

        let current_timestamp = Instant::now();
        let travel_to_the_past = current_timestamp.checked_sub(Duration::from_secs((MAX_AGE_CHAT_MESSAGES + 1) as u64));
        if travel_to_the_past.is_none() {
            warn!("skipping rest of test; cannot travel to the past :(");
            return;
        }
        let travel_to_the_past = travel_to_the_past.unwrap();
        for ref mut room in server.rooms.values_mut() {
            println!("Room: {:?}", room.name);
            for m in room.messages.iter_mut() {
                println!(
                    "{:?}, {:?},       {:?}",
                    m.timestamp,
                    travel_to_the_past,
                    m.timestamp - travel_to_the_past
                );
                m.timestamp = travel_to_the_past;
            }
        }

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms(time::Instant::now());

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }

    #[test]
    fn handle_new_connection_good_case() {
        let mut server = ServerState::new();
        let player_name = "some name".to_owned();
        let pkt = server.handle_new_connection(player_name, fake_socket_addr());
        match pkt {
            Packet::Response {
                sequence: _,
                request_ack: _,
                code,
            } => match code {
                ResponseCode::LoggedIn {
                    cookie: _,
                    server_version: _,
                } => {}
                _ => panic!("Unexpected ResponseCode: {:?}", code),
            },
            _ => panic!("Unexpected Packet Type: {:?}", pkt),
        }
    }

    #[test]
    fn handle_new_connection_player_name_taken() {
        let mut server = ServerState::new();
        let player_name = "some name".to_owned();

        let pkt = server.handle_new_connection(player_name.clone(), fake_socket_addr());
        match pkt {
            Packet::Response {
                sequence: _,
                request_ack: _,
                code,
            } => match code {
                ResponseCode::LoggedIn {
                    cookie: _,
                    server_version,
                } => assert_eq!(server_version, VERSION.to_owned()),
                _ => panic!("Unexpected ResponseCode: {:?}", code),
            },
            _ => panic!("Unexpected Packet Type: {:?}", pkt),
        }

        let pkt = server.handle_new_connection(player_name, fake_socket_addr());
        match pkt {
            Packet::Response {
                sequence: _,
                request_ack: _,
                code,
            } => match code {
                ResponseCode::Unauthorized { error_msg } => {
                    assert_eq!(error_msg, "not a unique name".to_owned());
                }
                _ => panic!("Unexpected ResponseCode: {:?}", code),
            },
            _ => panic!("Unexpected Packet Type: {:?}", pkt),
        }
    }

    fn a_request_action_strat() -> BoxedStrategy<RequestAction> {
        prop_oneof![
            //Just(RequestAction::Disconnect), // not yet implemented
            //Just(RequestAction::KeepAlive),  // same
            Just(RequestAction::LeaveRoom),
            Just(RequestAction::ListPlayers),
            Just(RequestAction::ListRooms),
            Just(RequestAction::None),
        ]
        .boxed()
    }

    fn a_request_action_complex_strat() -> BoxedStrategy<RequestAction> {
        prop_oneof![
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::ChatMessage { message: a }),
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::NewRoom { room_name: a }),
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::JoinRoom { room_name: a }),
            ("([A-Z]{1,4} [0-9]{1,2}){3}", "[0-9].[0-9].[0-9]").prop_map(|(a, b)| {
                RequestAction::Connect {
                    name:           a,
                    client_version: b,
                }
            })
        ]
        .boxed()
    }

    // These tests are checking that we do not panic on each RequestAction
    proptest! {
        #[test]
        fn process_request_action_simple(ref request in a_request_action_strat()) {
            let mut server = ServerState::new();
            server.create_new_room(None, "some room".to_owned().clone());
            let player_id: PlayerID = {
                let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
                player.player_id
            };
            server.process_request_action(player_id, request.to_owned());
        }

        #[test]
        fn process_request_action_complex(ref request in a_request_action_complex_strat()) {
            let mut server = ServerState::new();
            server.create_new_room(None, "some room".to_owned().clone());
            let player_id: PlayerID = {
                let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
                player.player_id
            };
            server.process_request_action(player_id, request.to_owned());
        }
    }

    #[test]
    fn process_request_action_connect_while_connected() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();
        server.create_new_room(None, "some room".to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        let result = server.process_request_action(
            player_id,
            RequestAction::Connect {
                name:           player_name,
                client_version: "0.1.0".to_owned(),
            },
        );
        assert_eq!(
            result,
            ResponseCode::BadRequest {
                error_msg: "Already connected".to_owned(),
            }
        );
    }

    #[test]
    fn process_request_action_none_is_invalid() {
        let mut server = ServerState::new();
        server.create_new_room(None, "some room".to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        let result = server.process_request_action(player_id, RequestAction::None);
        assert_eq!(
            result,
            ResponseCode::BadRequest {
                error_msg: "Invalid request: None".to_owned(),
            }
        );
    }

    #[test]
    fn prepare_response_spot_check_response_packet() {
        let mut server = ServerState::new();
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.request_ack = Some(1);
            player.player_id
        };
        let pkt: Packet = server.prepare_response(player_id, RequestAction::ListRooms).unwrap();
        match pkt {
            Packet::Response {
                code,
                sequence,
                request_ack,
            } => {
                if let ResponseCode::RoomList { rooms } = code {
                    assert_eq!(rooms.len(), 1); // 1 room - general
                } else {
                    panic!("`code` is not a RoomList! code is {:?}", code);
                }
                assert_eq!(sequence, 1);
                assert_eq!(request_ack, Some(2));
            }
            _ => panic!("Unexpected Packet type on Response path: {:?}", pkt),
        }
        let player: &Player = server.get_player(player_id);
        assert_eq!(player.next_resp_seq, 2);
    }

    #[test]
    fn validate_client_version_client_is_up_to_date() {
        assert_eq!(validate_client_version(env!("CARGO_PKG_VERSION").to_owned()), true);
    }

    #[test]
    fn validate_client_version_client_is_very_old() {
        assert_eq!(validate_client_version("0.0.1".to_owned()), true);
    }

    #[test]
    fn validate_client_version_client_is_from_the_future() {
        assert_eq!(
            validate_client_version(
                format!("{}.{}.{}", <i32>::max_value(), <i32>::max_value(), <i32>::max_value()).to_owned()
            ),
            false
        );
    }

    #[test]
    fn decode_packet_update_reply_good_case() {
        let mut server = ServerState::new();
        let cookie: String = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.cookie.clone()
        };

        // TODO: Move this into a private helper
        let update_reply_packet = Packet::UpdateReply {
            cookie:               cookie,
            last_chat_seq:        Some(0),
            last_game_update_seq: None,
            last_gen:             None,
            pong:                 PingPong::pong(0),
        };

        let result = server.decode_packet(fake_socket_addr(), update_reply_packet);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn decode_packet_update_reply_invalid_cookie() {
        let mut server = ServerState::new();
        {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.cookie.clone()
        };

        let cookie = "CookieMonster".to_owned();

        let update_reply_packet = Packet::UpdateReply {
            cookie:               cookie,
            last_chat_seq:        Some(0),
            last_game_update_seq: None,
            last_gen:             None,
            pong:                 PingPong::pong(0),
        };

        let result = server.decode_packet(fake_socket_addr(), update_reply_packet);
        assert!(result.is_err());
    }

    #[test]
    fn construct_client_updates_no_rooms() {
        let mut server = ServerState::new();
        let opt_updates = server.construct_client_updates();
        assert!(opt_updates.is_none());
    }

    #[test]
    fn construct_client_updates_empty_rooms() {
        let mut server = ServerState::new();
        server.create_new_room(None, "some room".to_owned().clone());
        let opt_updates = server.construct_client_updates();
        assert!(opt_updates.is_none());
    }

    #[test]
    fn construct_client_updates_populated_room_returns_all_messages() {
        let mut server = ServerState::new();
        let room_name = "some_room";
        let player_name = "some player".to_owned();
        let message_text = "Message".to_owned();

        server.create_new_room(None, room_name.to_owned());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        server.join_room(player_id, room_name);
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());

        let opt_updates = server.construct_client_updates();
        assert!(opt_updates.is_some());

        let mut output: Vec<(SocketAddr, Packet)> = opt_updates.unwrap();

        // Vector should contain a single item for this test
        assert_eq!(output.len(), 1);

        let (addr, pkt) = output.pop().unwrap();
        assert_eq!(addr, fake_socket_addr());

        match pkt {
            Packet::Update {
                chats,
                game_updates,
                universe_update,
                ping: _,
            } => {
                assert!(game_updates.is_empty());
                assert_eq!(universe_update, UniUpdateType::NoChange);
                assert!(!chats.is_empty());

                // All client chat sequence numbers start counting at 1
                let mut i = 1;

                for msg in chats {
                    assert_eq!(msg.player_name, player_name);
                    assert_eq!(msg.chat_seq, Some(i));
                    assert_eq!(msg.message, message_text);
                    i += 1;
                }
            }
            _ => panic!("Unexpected packet in client update construction!"),
        }
    }

    #[test]
    fn construct_client_updates_populated_room_returns_updates_after_client_acked() {
        let mut server = ServerState::new();
        let room_name = "some_room";
        let player_name = "some player".to_owned();
        let message_text = "Message".to_owned();

        server.create_new_room(None, room_name.to_owned());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        server.join_room(player_id, room_name);
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());

        // Assume that the client has acknowledged two chats
        {
            let player: &mut Player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(2));
        }

        // We should then only return the last chat
        let opt_updates = server.construct_client_updates();

        assert!(opt_updates.is_some());
        let mut output: Vec<(SocketAddr, Packet)> = opt_updates.unwrap();

        // Vector should contain a single item for this test
        assert_eq!(output.len(), 1);

        let (addr, pkt) = output.pop().unwrap();
        assert_eq!(addr, fake_socket_addr());

        match pkt {
            Packet::Update {
                mut chats,
                game_updates,
                universe_update,
                ping: _,
            } => {
                assert!(game_updates.is_empty());
                assert_eq!(universe_update, UniUpdateType::NoChange);
                assert!(!chats.is_empty());

                assert_eq!(chats.len(), 1);
                let msg = chats.pop().unwrap();

                assert_eq!(msg.player_name, player_name);
                assert_eq!(msg.chat_seq, Some(3));
                assert_eq!(msg.message, message_text);
            }
            _ => panic!("Unexpected packet in client update construction!"),
        }
    }

    #[test]
    fn broadcast_message_to_two_players_in_room() {
        let mut server = ServerState::new();
        let room_name = "some_room";
        let player_name = "some player".to_owned();

        server.create_new_room(None, room_name.to_owned());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.clone());
        {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.broadcast("Silver birch against a Swedish sky".to_owned());
        }
        server.join_room(player_id2, room_name);
        let room: &Room = server.get_room(player_id).unwrap();

        let player = (*server.get_player(player_id)).clone();
        let msgs = server.collect_unacknowledged_messages(room, &player).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message, "Silver birch against a Swedish sky".to_owned());

        let player = (*server.get_player(player_id2)).clone();
        let msgs = server.collect_unacknowledged_messages(room, &player).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message, "Silver birch against a Swedish sky".to_owned());
    }

    #[test]
    fn broadcast_message_to_an_empty_room() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();

        server.create_new_room(None, room_name.clone());
        let room_id: &RoomID = server.room_map.get(&room_name.clone()).unwrap();

        {
            let room: &mut Room = server.rooms.get_mut(&room_id).unwrap();
            room.broadcast("Silver birch against a Swedish sky".to_owned());
        }
        let room: &Room = server.rooms.get(&room_id).unwrap();
        assert_eq!(room.latest_seq_num, 1);
        assert_eq!(room.messages.len(), 1);
        let msgs: &ServerChatMessage = room.messages.get(0).unwrap();
        assert_eq!(msgs.player_name, "Server".to_owned());
        assert_eq!(msgs.seq_num, 1);
        assert_eq!(msgs.player_id, PlayerID(0xFFFFFFFFFFFFFFFF));
    }

    #[test]
    #[should_panic]
    fn disconnect_get_player_by_id_fails() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.handle_disconnect(player_id);
        server.get_player(player_id);
    }

    #[test]
    fn disconnect_get_player_by_cookie_fails() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, cookie) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            (player.player_id, player.cookie.clone())
        };

        server.handle_disconnect(player_id);
        assert_eq!(server.get_player_id_by_cookie(cookie.as_str()), None);
    }

    #[test]
    fn disconnect_while_in_room_removes_all_traces_of_player() {
        let mut server = ServerState::new();
        let room_name = "some_room";
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.create_new_room(None, room_name.to_owned());
        server.join_room(player_id, room_name);
        let room_id = {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.player_ids.contains(&player_id), true);
            room.room_id
        };
        server.handle_disconnect(player_id);
        // Cannot go through player_id because the player has been removed
        let room: &Room = server.rooms.get(&room_id).unwrap();
        assert_eq!(room.player_ids.contains(&player_id), false);
    }

    #[test]
    fn test_is_previously_processed_packet() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(4);
            player.player_id
        };

        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = None;
            player.player_id
        };

        assert_eq!(server.is_previously_processed_packet(player_id2, 0), false);

        assert_eq!(server.is_previously_processed_packet(player_id, 0), true);
        assert_eq!(server.is_previously_processed_packet(player_id, 4), true);
        assert_eq!(server.is_previously_processed_packet(player_id, 5), false);
    }

    #[test]
    fn test_clear_transmission_queue_on_ack() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(4);
            player.player_id
        };

        for i in 0..5 {
            let pkt = Packet::Response {
                sequence:    i,
                request_ack: None,
                code:        ResponseCode::OK,
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.tx_packets.buffer_item(pkt.clone());
        }

        server.clear_transmission_queue_on_ack(player_id, None);
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 5);
        server.clear_transmission_queue_on_ack(player_id, Some(0));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 5);
        server.clear_transmission_queue_on_ack(player_id, Some(1));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 4);
        server.clear_transmission_queue_on_ack(player_id, Some(5));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 0);
    }

    #[test]
    fn test_resend_expired_tx_packets_empty_server() {
        let mut server = ServerState::new();

        let (udp_tx, _) = mpsc::unbounded();
        #[cfg(not(should_panic))]
        server.resend_expired_tx_packets(&udp_tx);
    }

    #[test]
    fn test_resend_expired_tx_packets() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(5);
            player.player_id
        };

        for i in 0..5 {
            let pkt = Packet::Response {
                sequence:    i,
                request_ack: None,
                code:        ResponseCode::OK,
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.tx_packets.buffer_item(pkt.clone());

            if i < 3 {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.back_mut().unwrap();
                attempt.time = Instant::now() - Duration::from_secs(i + 1);
            }
        }

        let (udp_tx, _) = mpsc::unbounded();
        server.resend_expired_tx_packets(&udp_tx);

        for i in 0..5 {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            let packet_retries: &NetAttempt = nm.tx_packets.attempts.get(i).unwrap();

            if i >= 3 {
                assert_eq!(packet_retries.retries, 0);
            } else {
                assert_eq!(packet_retries.retries, 1);
            }
        }
    }

    #[test]
    fn test_process_queued_rx_packets_first_non_connect_player_packet() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1); // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        {
            let pkt = Packet::Request {
                cookie:       Some(player_cookie),
                sequence:     2,
                response_ack: None,
                action:       RequestAction::ListPlayers,
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);

        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 1);
        }
    }

    #[test]
    fn test_process_queued_rx_packets_contiguous() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1); // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        for i in 2..10 {
            let pkt = Packet::Request {
                cookie:       Some(player_cookie.clone()),
                sequence:     i,
                response_ack: None,
                action:       RequestAction::ListPlayers,
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);

        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 8);
        }
    }

    #[test]
    fn test_process_queued_rx_packets_swiss_cheese_queue() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1); // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        for i in [2, 3, 4, 6, 8, 9, 10].iter() {
            let pkt = Packet::Request {
                cookie:       Some(player_cookie.clone()),
                sequence:     *i,
                response_ack: None,
                action:       RequestAction::ListPlayers,
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);

        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 3); // only 2, 3, and 4 are processed
        }
    }
}
