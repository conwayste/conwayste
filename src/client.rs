/*
 * A networking library for the multiplayer game, Conwayste.
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

use std::error::Error;
use std::net::SocketAddr;
use std::time::Instant;

use crate::net::{
    RequestAction, ResponseCode, Packet,
    BroadcastChatMessage, NetworkManager, NetworkQueue,
    VERSION, has_connection_timed_out
};

use crate::futures::sync::mpsc;

pub const CLIENT_VERSION: &str = "0.0.1";

pub struct ClientState {
    pub sequence:         u64,          // Sequence number of requests
    pub response_sequence: u64,         // Value of the next expected sequence number from the server,
                                        // and indicates the sequence number of the next process-able rx packet
    pub name:             Option<String>,
    pub room:             Option<String>,
    pub cookie:           Option<String>,
    pub chat_msg_seq_num: u64,
    pub tick:             usize,
    pub network:          NetworkManager,
    pub heartbeat:        Option<Instant>,
    pub disconnect_initiated: bool,
}

impl ClientState {

    pub fn new() -> Self {
        ClientState {
            sequence:        0,
            response_sequence: 0,
            name:            None,
            room:            None,
            cookie:          None,
            chat_msg_seq_num: 0,
            tick:            0,
            network:         NetworkManager::new().with_message_buffering(),
            heartbeat:       None,
            disconnect_initiated: false,
        }
    }

    pub fn reset(&mut self) {
        // Design pattern taken from https://blog.getseq.net/rust-at-datalust-how-we-organize-a-complex-rust-codebase/
        // The intention is that new fields added to ClientState will cause compiler errors unless
        // we add them here.
        #![deny(unused_variables)]
        let Self {
            ref mut sequence,
            ref mut response_sequence,
            name: ref _name,
            ref mut room,
            ref mut cookie,
            ref mut chat_msg_seq_num,
            ref mut tick,
            ref mut network,
            ref mut heartbeat,
            ref mut disconnect_initiated,
        } = *self;
        *sequence         = 0;
        *response_sequence = 0;
        *room             = None;
        *cookie           = None;
        *chat_msg_seq_num = 0;
        *tick             = 0;
        network.reset();
        *heartbeat        = None;
        *disconnect_initiated = false;

        trace!("ClientState reset!");
    }

    fn print_help() {
        println!("");
        println!("/help                  - print this text");
        println!("/connect <player_name> - connect to server");
        println!("/disconnect            - disconnect from server");
        println!("/list                  - list rooms when in lobby, or players when in game");
        println!("/new <room_name>       - create a new room (when not in game)");
        println!("/join <room_name>      - join a room (when not in game)");
        println!("/leave                 - leave a room (when in game)");
        println!("/quit                  - exit the program");
        println!("...or just type text to chat!");
    }

    pub fn in_game(&self) -> bool {
        self.room.is_some()
    }

    // XXX Once netwayste integration is complete, we'll need to internally send
    // the result of most of these handlers so we can notify a player via UI event.

    pub fn check_for_upgrade(&self, server_version: &String) {
        let client_version = &VERSION.to_owned();
        if client_version < server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nnWarning: Client out-of-date. Please upgrade.", client_version, server_version);
        }
        else if client_version > server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nWarning: Client Version greater than Server Version.", client_version, server_version);
        }
    }

    pub fn process_queued_server_responses(&mut self) {
        // If we can, start popping off the RX queue and handle contiguous packets immediately
        let mut dequeue_count = 0;

        let rx_queue_count = self.network.rx_packets.get_contiguous_packets_count(self.response_sequence);
        while dequeue_count < rx_queue_count {
            let packet = self.network.rx_packets.as_queue_type_mut().pop_front().unwrap();
            trace!("{:?}", packet);
            match packet {
                Packet::Response{sequence: _, request_ack: _, code} => {
                    dequeue_count += 1;
                    self.response_sequence += 1;
                    self.process_event_code(code);
                }
                _ => panic!("Development bug: Non-response packet found in client RX queue")
            }
        }
    }

    pub fn process_event_code(&mut self, code: ResponseCode) {
        match code {
            ResponseCode::OK => {
                match self.handle_response_ok() {
                    Ok(_) => {},
                    Err(e) => error!("{:?}", e)
                }
            }
            ResponseCode::LoggedIn(ref cookie, ref server_version) => {
                self.handle_logged_in(cookie.to_string(), server_version.to_string());
            }
            ResponseCode::LeaveRoom => {
                self.handle_left_room();
            }
            ResponseCode::JoinedRoom(ref room_name) => {
                self.handle_joined_room(room_name);
            }
            ResponseCode::PlayerList(ref player_names) => {
                self.handle_player_list(player_names.to_vec());
            }
            ResponseCode::RoomList(ref rooms) => {
                self.handle_room_list(rooms.to_vec());
            }
            ResponseCode::KeepAlive => {
                self.heartbeat = Some(Instant::now());
            },
            // errors
            _ => {
                error!("unknown response from server: {:?}", code);
            }
        }
    }

    pub fn handle_incoming_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr, opt_packet: Option<Packet>) {
        // All `None` packets should get filtered out up the hierarchy
        let packet = opt_packet.unwrap();
        match packet.clone() {
            Packet::Response{sequence, request_ack: _, code} => {
                self.process_event_code(ResponseCode::KeepAlive); // On any incoming event update the heartbeat.
                let code = code.clone();
                if code != ResponseCode::KeepAlive {
                    // When a packet is acked, we can remove it from the TX buffer and buffer the response for
                    // later processing.
                    // Removing a "Response packet" from the client's request TX buffer appears to be nonsense at first.
                    // This works because remove() targets different ID's depending on the Packet type. In the case of
                    // a Response packet, the target identifier is the `request_ack`.

                    // Only process responses we haven't seen
                    if self.response_sequence <= sequence {
                        trace!("RX Buffering: Resp.Seq.: {}, {:?}", self.response_sequence, packet);
                        // println!("TX packets: {:?}", self.network.tx_packets);
                        // None means the packet was not found so we've probably already removed it.
                        if let Some(_) = self.network.tx_packets.remove(&packet)
                        {
                            self.network.rx_packets.buffer_item(packet);
                        }

                        self.process_queued_server_responses();
                    }
                }
            }
            // TODO game_updates, universe_update
            Packet::Update{chats, game_updates: _, universe_update: _} => {
                self.handle_incoming_chats(chats);

                // Reply to the update
                let packet = Packet::UpdateReply {
                    cookie:               self.cookie.clone().unwrap(),
                    last_chat_seq:        Some(self.chat_msg_seq_num),
                    last_game_update_seq: None,
                    last_gen:             None,
                };

                netwayste_send!(udp_tx, (addr.clone(), packet),
                         ("Could not send UpdateReply{{ {} }} to server", self.chat_msg_seq_num));
            }
            Packet::Request{..} => {
                warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
            }
            Packet::UpdateReply{..} => {
                warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
            }
        }
    }

    pub fn handle_network_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr) {
        if self.cookie.is_some() {
            // Determine what can be processed
            // Determine what needs to be resent
            // Resend anything remaining in TX queue if it has also expired.
            self.process_queued_server_responses();

            let indices = self.network.tx_packets.get_retransmit_indices();

            self.network.retransmit_expired_tx_packets(udp_tx, addr, Some(self.response_sequence), &indices);
        }
    }

    pub fn handle_tick_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr) {
        // Every 100ms, after we've connected
        if self.cookie.is_some() {
            // Send a keep alive heartbeat if the connection is live
            let keep_alive = Packet::Request {
                cookie: self.cookie.clone(),
                sequence: self.sequence,
                response_ack: None,
                action: RequestAction::KeepAlive(self.response_sequence),
            };
            let timed_out = has_connection_timed_out(self.heartbeat);

            if timed_out || self.disconnect_initiated {
                if timed_out {
                    trace!("Server is non-responsive, disconnecting.");
                }
                if self.disconnect_initiated {
                    trace!("Disconnected from the server.")
                }
                self.reset();
            } else {
                netwayste_send!(udp_tx, (addr, keep_alive), ("Could not send KeepAlive packets"));
            }
        }

        self.tick = 1usize.wrapping_add(self.tick);
    }

    pub fn handle_user_input_event(&mut self,
            udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>,
            exit_tx: &mpsc::UnboundedSender<()>,
            user_input: UserInput,
            addr: SocketAddr) {

        let action;
        match user_input {
            UserInput::Chat(string) => {
                action = RequestAction::ChatMessage(string);
            }
            UserInput::Command{cmd, args} => {
                action = self.build_command_request_action(cmd, args);
            }
        }
        if action != RequestAction::None {
            // Sequence number can increment once we're talking to a server
            if self.cookie != None {
                self.sequence += 1;
            }

            let packet = Packet::Request {
                sequence:     self.sequence,
                response_ack: Some(self.response_sequence),
                cookie:       self.cookie.clone(),
                action:       action.clone(),
            };

            trace!("{:?}", packet);

            self.network.tx_packets.buffer_item(packet.clone());

            netwayste_send!(udp_tx, (addr.clone(), packet),
                            ("Could not send user input cmd to server"));

            if action == RequestAction::Disconnect {
                self.disconnect_initiated = true;
                netwayste_send!(exit_tx, ());
            }
        }
    }

    pub fn handle_response_ok(&mut self) -> Result<(), Box<Error>> {
            info!("OK :)");
            return Ok(());
    }

    pub fn handle_logged_in(&mut self, cookie: String, server_version: String) {
        self.cookie = Some(cookie);

        info!("Set client name to {:?}", self.name.clone().unwrap());
        self.check_for_upgrade(&server_version);
    }

    pub fn handle_joined_room(&mut self, room_name: &String) {
            self.room = Some(room_name.clone());
            info!("Joined room: {}", room_name);
    }

    pub fn handle_left_room(&mut self) {
        if self.in_game() {
            info!("Left room {}.", self.room.clone().unwrap());
        }
        self.room = None;
        self.chat_msg_seq_num = 0;
    }

    pub fn handle_player_list(&mut self, player_names: Vec<String>) {
        info!("---BEGIN PLAYER LIST---");
        for (i, player_name) in player_names.iter().enumerate() {
            info!("{}\tname: {}", i, player_name);
        }
        info!("---END PLAYER LIST---");
    }

    pub fn handle_room_list(&mut self, rooms: Vec<(String, u64, bool)>) {
        info!("---BEGIN GAME ROOM LIST---");
        for (game_name, num_players, game_running) in rooms {
            info!("#players: {},\trunning? {:?},\tname: {:?}",
                        num_players,
                        game_running,
                        game_name);
        }
        info!("---END GAME ROOM LIST---");
    }

    pub fn handle_incoming_chats(&mut self, chats: Option<Vec<BroadcastChatMessage>> ) {
        if let Some(mut chat_messages) = chats {
            chat_messages.retain(|ref chat_message| {
                self.chat_msg_seq_num < chat_message.chat_seq.unwrap()
            });
            // This loop does two things:
            //  1) update chat_msg_seq_num, and
            //  2) prints messages from other players
            for chat_message in chat_messages {
                let chat_seq = chat_message.chat_seq.unwrap();
                self.chat_msg_seq_num = std::cmp::max(chat_seq, self.chat_msg_seq_num);

                let queue = self.network.rx_chat_messages.as_mut().unwrap();
                queue.buffer_item(chat_message.clone());

                if let Some(ref client_name) = self.name.as_ref() {
                    if *client_name != &chat_message.player_name {
                        info!("{}: {}", chat_message.player_name, chat_message.message);
                    }
                } else {
                   panic!("Client name not set!");
                }
            }
        }
    }

    pub fn build_command_request_action(&mut self, cmd: String, args: Vec<String>) -> RequestAction {
        let mut action: RequestAction = RequestAction::None;
        // keep these in sync with print_help function
        match cmd.as_str() {
            "help" => {
                ClientState::print_help();
            }
            "stats" => {
                self.network.print_statistics();
            }
            "connect" => {
                if args.len() == 1 {
                    self.name = Some(args[0].clone());
                    action = RequestAction::Connect{
                        name:           args[0].clone(),
                        client_version: CLIENT_VERSION.to_owned(),
                    };
                } else { error!("Expected client name as the sole argument (no spaces allowed)."); }
            }
            "disconnect" => {
                if args.len() == 0 {
                    action = RequestAction::Disconnect;
                } else { debug!("Command failed: Expected no arguments to disconnect"); }
            }
            "list" => {
                if args.len() == 0 {
                    // players or rooms
                    if self.in_game() {
                        action = RequestAction::ListPlayers;
                    } else {
                        // lobby
                        action = RequestAction::ListRooms;
                    }
                } else { debug!("Command failed: Expected no arguments to list"); }
            }
            "new" => {
                if args.len() == 1 {
                    action = RequestAction::NewRoom(args[0].clone());
                } else { debug!("Command failed: Expected name of room (no spaces allowed)"); }
            }
            "join" => {
                if args.len() == 1 {
                    if !self.in_game() {
                        action = RequestAction::JoinRoom(args[0].clone());
                    } else {
                        debug!("Command failed: You are already in a game");
                    }
                } else { debug!("Command failed: Expected room name only (no spaces allowed)"); }
            }
            "part" | "leave" => {
                if args.len() == 0 {
                    if self.in_game() {
                        action = RequestAction::LeaveRoom;
                    } else {
                        debug!("Command failed: You are already in the lobby");
                    }
                } else { debug!("Command failed: Expected no arguments to leave"); }
            }
            "quit" => {
                trace!("Peace out!");
                action = RequestAction::Disconnect;
            }
            _ => {
                debug!("Command not recognized: {}", cmd);
            }
        }
        return action;
    }
}

// At this point we should only have command or chat message to work with
pub fn parse_stdin(mut input: String) -> UserInput {
    if input.get(0..1) == Some("/") {
        // this is a command
        input.remove(0);  // remove initial slash

        let mut words: Vec<String> = input.split_whitespace().map(|w| w.to_owned()).collect();

        let command = if words.len() > 0 {
                        words.remove(0).to_lowercase()
                    } else {
                        "".to_owned()
                    };

        UserInput::Command{cmd: command, args: words}
    } else {
            UserInput::Chat(input)
    }
}

//////////////// Event Handling /////////////////
#[derive(PartialEq, Debug, Clone)]
pub enum UserInput {
    Command{cmd: String, args: Vec<String>},
    Chat(String),
}
