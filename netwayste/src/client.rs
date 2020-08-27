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

use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use std::time::Instant;

use futures as Fut;
use regex::Regex;
use tokio::time as TokioTime;
use tokio_util::udp::UdpFramed;
use Fut::prelude::*;
use Fut::select;

use crate::net::{
    bind, has_connection_timed_out, BroadcastChatMessage, NetwaysteEvent, NetwaystePacketCodec, NetworkManager,
    NetworkQueue, Packet, RequestAction, ResponseCode, RoomList, DEFAULT_PORT, VERSION,
};

use crate::utils::{LatencyFilter, PingPong};

const TICK_INTERVAL_IN_MS: u64 = 1000;
const NETWORK_INTERVAL_IN_MS: u64 = 1000;

pub const CLIENT_VERSION: &str = "0.0.1";

pub struct ClientNetState {
    pub sequence:             u64, // Sequence number of requests
    pub response_sequence:    u64, // Value of the next expected sequence number from the server,
    // and indicates the sequence number of the next process-able rx packet
    pub name:                 Option<String>,
    pub room:                 Option<String>,
    pub cookie:               Option<String>,
    pub chat_msg_seq_num:     u64,
    pub tick:                 usize,
    pub network:              NetworkManager,
    pub last_received:        Option<Instant>,
    pub disconnect_initiated: bool,
    pub server_address:       Option<SocketAddr>,
    pub channel_to_conwayste: Fut::channel::mpsc::Sender<NetwaysteEvent>,
    latency_filter:           LatencyFilter,
}

impl ClientNetState {
    pub fn new(channel_to_conwayste: Fut::channel::mpsc::Sender<NetwaysteEvent>) -> Self {
        ClientNetState {
            sequence:             0,
            response_sequence:    0,
            name:                 None,
            room:                 None,
            cookie:               None,
            chat_msg_seq_num:     0,
            tick:                 0,
            network:              NetworkManager::new().with_message_buffering(),
            last_received:        None,
            disconnect_initiated: false,
            server_address:       None,
            channel_to_conwayste: channel_to_conwayste,
            latency_filter:       LatencyFilter::new(),
        }
    }

    pub fn reset(&mut self) {
        // Design pattern taken from https://blog.getseq.net/rust-at-datalust-how-we-organize-a-complex-rust-codebase/
        // The intention is that new fields added to ClientNetState will cause compiler errors unless
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
            ref mut last_received,
            ref mut disconnect_initiated,
            ref mut server_address,
            channel_to_conwayste: ref _channel_to_conwayste, // Don't clear the channel to conwayste
            ref mut latency_filter,
        } = *self;
        *sequence = 0;
        *response_sequence = 0;
        *room = None;
        *cookie = None;
        *chat_msg_seq_num = 0;
        *tick = 0;
        *last_received = None;
        *disconnect_initiated = false;
        *server_address = None;
        network.reset();
        latency_filter.reset();

        trace!("ClientNetState reset!");
    }

    pub fn in_game(&self) -> bool {
        self.room.is_some()
    }

    fn check_for_upgrade(&self, server_version: &String) {
        let client_version = &VERSION.to_owned();
        if client_version < server_version {
            warn!(
                "\tClient Version: {}\n\tServer Version: {}\nnWarning: Client out-of-date. Please upgrade.",
                client_version, server_version
            );
        } else if client_version > server_version {
            warn!(
                "\tClient Version: {}\n\tServer Version: {}\nWarning: Client Version greater than Server Version.",
                client_version, server_version
            );
        }
    }

    async fn process_queued_server_responses(&mut self) {
        // If we can, start popping off the RX queue and handle contiguous packets immediately
        let mut dequeue_count = 0;

        let rx_queue_count = self
            .network
            .rx_packets
            .get_contiguous_packets_count(self.response_sequence);
        while dequeue_count < rx_queue_count {
            let packet = self.network.rx_packets.as_queue_type_mut().pop_front().unwrap();
            trace!("{:?}", packet);
            match packet {
                Packet::Response {
                    sequence: _,
                    request_ack: _,
                    code,
                } => {
                    dequeue_count += 1;
                    self.response_sequence += 1;
                    self.process_event_code(code).await;
                }
                _ => panic!("Development bug: Non-response packet found in client RX queue"),
            }
        }
    }

    async fn process_event_code(&mut self, code: ResponseCode) {
        match code.clone() {
            ResponseCode::OK => match self.handle_response_ok() {
                Ok(_) => {}
                Err(e) => error!("{:?}", e),
            },
            ResponseCode::LoggedIn {
                ref cookie,
                ref server_version,
            } => {
                self.handle_logged_in(cookie.to_string(), server_version.to_string());
            }
            ResponseCode::LeaveRoom => {
                self.handle_left_room();
            }
            ResponseCode::JoinedRoom { ref room_name } => {
                self.handle_joined_room(room_name);
            }
            ResponseCode::PlayerList { ref players } => {
                self.handle_player_list(players.to_vec());
            }
            ResponseCode::RoomList { ref rooms } => {
                self.handle_room_list(rooms.to_vec());
            }
            ResponseCode::KeepAlive => {}
            // errors
            ResponseCode::Unauthorized { error_msg: opt_error } => {
                info!("Unauthorized action attempted by client: {:?}", opt_error);
            }
            _ => {
                error!("unknown response from server: {:?}", code);
            }
        }

        if code != ResponseCode::OK && code != ResponseCode::KeepAlive {
            let nw_response: NetwaysteEvent = NetwaysteEvent::build_netwayste_event_from_response_code(code);
            match self.channel_to_conwayste.send(nw_response).await {
                Ok(_) => (),
                Err(e) => error!("Could not send a netwayste response via channel_to_conwayste: {:?}", e),
            }
        }
    }

    pub async fn handle_incoming_event(&mut self, packet: Packet, addr: SocketAddr) -> Vec<(Packet, SocketAddr)> {
        match packet.clone() {
            Packet::Response {
                sequence,
                request_ack: _,
                code,
            } => {
                self.last_received = Some(Instant::now());
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
                        if let Some(_) = self.network.tx_packets.remove(&packet) {
                            self.network.rx_packets.buffer_item(packet);
                        }

                        self.process_queued_server_responses().await;
                    }
                }
                return vec![];
            }
            // TODO game_updates, universe_update
            Packet::Update {
                chats,
                game_updates: _,
                universe_update: _,
                ping,
            } => {
                if chats.len() != 0 {
                    self.handle_incoming_chats(chats).await;
                }

                // Reply to the update
                let update_reply_packet = Packet::UpdateReply {
                    cookie:               self.cookie.clone().unwrap(),
                    last_chat_seq:        Some(self.chat_msg_seq_num),
                    last_game_update_seq: None,
                    last_gen:             None,
                    pong:                 PingPong::pong(ping.nonce),
                };

                return vec![(update_reply_packet, addr)];
            }
            Packet::Request { .. } | Packet::UpdateReply { .. } | Packet::GetStatus { .. } => {
                warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
                return vec![];
            }
            Packet::Status { .. } => {
                self.latency_filter.update();

                self.channel_to_conwayste
                    .send(NetwaysteEvent::Status(packet, self.latency_filter.average_latency_ms))
                    .await
                    .unwrap_or_else(|e| {
                        error!("Could not send a netwayste response via channel_to_conwayste: {:?}", e);
                    });
                return vec![];
            }
        }
    }

    pub async fn collect_expired_tx_packets(&mut self) -> Vec<(Packet, SocketAddr)> {
        if self.cookie.is_some() {
            // Determine what can be processed
            // Determine what needs to be resent
            // Resend anything remaining in TX queue if it has also expired.
            self.process_queued_server_responses().await;

            let indices = self.network.tx_packets.get_retransmit_indices();

            return self.network.get_expired_tx_packets(
                self.server_address.unwrap().clone(),
                Some(self.response_sequence),
                &indices,
            );
        }
        return vec![];
    }

    fn handle_tick_event(&mut self) -> Option<Packet> {
        // Every 100ms, after we've connected
        if self.cookie.is_some() {
            let timed_out = has_connection_timed_out(self.last_received.unwrap());

            if timed_out || self.disconnect_initiated {
                if timed_out {
                    info!("Server is non-responsive, disconnecting.");
                }
                if self.disconnect_initiated {
                    info!("Disconnected from the server.")
                }
                self.reset();
                return None;
            } else {
                // Send a keep alive if the connection is live
                let keep_alive = Packet::Request {
                    cookie:       self.cookie.clone(),
                    sequence:     self.sequence,
                    response_ack: None,
                    action:       RequestAction::KeepAlive {
                        latest_response_ack: self.response_sequence,
                    },
                };
                return Some(keep_alive);
            }
        }

        self.tick = 1usize.wrapping_add(self.tick);
        None
    }

    pub fn handle_response_ok(&mut self) -> Result<(), Box<dyn Error>> {
        info!("OK :)");
        return Ok(());
    }

    pub fn handle_logged_in(&mut self, cookie: String, server_version: String) {
        self.cookie = Some(cookie);

        if let Some(name) = self.name.as_ref() {
            info!("Logged in with client name {:?}", name);
        } else {
            warn!("Logged in, but no name set!");
        }
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

    pub fn handle_room_list(&mut self, rooms: Vec<RoomList>) {
        info!("---BEGIN GAME ROOM LIST---");
        for room in rooms {
            info!(
                "#name: {},\trunning? {:?},\tplayers: {:?}",
                room.room_name, room.in_progress, room.player_count
            );
        }
        info!("---END GAME ROOM LIST---");
    }

    pub async fn handle_incoming_chats(&mut self, mut chat_messages: Vec<BroadcastChatMessage>) {
        chat_messages.retain(|ref chat_message| self.chat_msg_seq_num < chat_message.chat_seq.unwrap());

        let mut to_conwayste_msgs = vec![];

        // This loop does three things:
        //  1) update chat_msg_seq_num, and
        //  2) prints messages from other players
        //  3) Transmits chats to conwayste
        for chat_message in chat_messages {
            let chat_seq = chat_message.chat_seq.unwrap();
            self.chat_msg_seq_num = std::cmp::max(chat_seq, self.chat_msg_seq_num);

            let queue = self.network.rx_chat_messages.as_mut().unwrap();
            queue.buffer_item(chat_message.clone());

            if let Some(client_name) = self.name.as_ref() {
                if client_name != &chat_message.player_name {
                    info!("{}: {}", chat_message.player_name, chat_message.message);
                    to_conwayste_msgs.push((chat_message.player_name, chat_message.message));
                }
            } else {
                panic!("Client name not set!");
            }
        }

        let nw_response = NetwaysteEvent::ChatMessages(to_conwayste_msgs);
        match self.channel_to_conwayste.send(nw_response).await {
            Ok(_) => (),
            Err(e) => error!("Could not send a netwayste response via channel_to_conwayste: {:?}", e),
        }
    }

    /// Prepare a request action to the connected server
    fn action_to_packet(&mut self, action: RequestAction) -> Packet {
        // Sequence number can increment once we're talking to a server
        if self.cookie != None {
            self.sequence += 1;
        }

        if action == RequestAction::Disconnect {
            // TODO: we don't necessarily want the netwayste thread to exit when we Disconnect
            // from a server!
            self.disconnect_initiated = true;
        }

        let packet = Packet::Request {
            sequence:     self.sequence,
            response_ack: Some(self.response_sequence),
            cookie:       self.cookie.clone(),
            action:       action,
        };

        trace!("{:?}", packet);

        self.network.tx_packets.buffer_item(packet.clone());

        packet
    }

    async fn maintain_network_state(&mut self) -> Vec<(Packet, SocketAddr)> {
        self.collect_expired_tx_packets().await
    }

    /// Main executor for the client-side network layer for conwayste and should be run from a thread.
    /// Its two arguments are halves of a channel used for communication to send and receive Netwayste events.
    pub async fn start_network(
        channel_to_conwayste: Fut::channel::mpsc::Sender<NetwaysteEvent>,
        mut channel_from_conwayste: Fut::channel::mpsc::UnboundedReceiver<NetwaysteEvent>,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let has_port_re = Regex::new(r":\d{1,5}$").unwrap(); // match a colon followed by number up to 5 digits (16-bit port)
        let mut server_str = env::args().nth(1).unwrap_or("localhost".to_owned());

        // if no port, add the default port
        if !has_port_re.is_match(&server_str) {
            debug!("Appending default port to {:?}", server_str);
            server_str = format!("{}:{}", server_str, DEFAULT_PORT);
        }

        let addr_iter = tokio::net::lookup_host(server_str).await?;
        let addr_vec: Vec<SocketAddr> = addr_iter.collect();

        let addresses_resolved = addr_vec.len();
        if addresses_resolved == 0 {
            error!("DNS resolution found 0 addresses");
            exit(1);
        }

        // TODO: support IPv6
        // filter out IPv6
        let v4_addr_vec: Vec<_> = addr_vec.into_iter().filter(|addr| addr.is_ipv4()).collect();
        if v4_addr_vec.len() < addresses_resolved {
            warn!(
                "Filtered out {} IPv6 addresses -- IPv6 is not implemented.",
                addresses_resolved - v4_addr_vec.len()
            );
        }
        if v4_addr_vec.len() > 1 {
            // This is probably not the best option -- could pick based on latency time, random choice,
            // and could also try other ones on connection failure.
            warn!(
                "Multiple ({:?}) addresses returned; arbitrarily picking the first one.",
                v4_addr_vec.len()
            );
        }

        let addr = v4_addr_vec[0];

        trace!("Connecting to {:?}", addr);

        // Unwrap ok because bind will abort if unsuccessful
        let udp = bind(Some("0.0.0.0"), Some(0)).await.unwrap_or_else(|e| {
            error!("Error while trying to bind UDP socket: {:?}", e);
            exit(1)
        });

        let local_addr = udp.local_addr()?;

        // Channels
        let (mut udp_sink, udp_stream) = UdpFramed::new(udp, NetwaystePacketCodec).split();
        let mut udp_stream = udp_stream.fuse();

        trace!("Locally bound to {:?}.", local_addr);
        trace!("Will connect to remote {:?}.", addr);

        // initialize state
        let mut client_state = ClientNetState::new(channel_to_conwayste);
        client_state.server_address = Some(addr);

        let mut tick_interval = TokioTime::interval(Duration::from_millis(TICK_INTERVAL_IN_MS)).fuse();
        let mut network_interval = TokioTime::interval(Duration::from_millis(NETWORK_INTERVAL_IN_MS)).fuse();

        loop {
            select! {
                (_) = tick_interval.select_next_some() => {
                    if let Some(keep_alive_pkt) = client_state.handle_tick_event() {
                        // Unwrap safe b/c the connection to server is active
                        udp_sink.send((keep_alive_pkt, client_state.server_address.unwrap())).await?;
                    }
                },
                (_) = network_interval.select_next_some() => {
                    let retransmissions = client_state.maintain_network_state().await;
                    for packet_addr_tuple in retransmissions {
                        udp_sink.send(packet_addr_tuple).await?;
                    }
                },
                (addr_packet_result) = udp_stream.select_next_some() => {
                    if let Ok((packet, addr)) = addr_packet_result {
                        let responses = client_state.handle_incoming_event(packet, addr).await;
                        for response in responses {
                            udp_sink.send(response).await?;
                        }
                    }
                },
                (netwayste_request) = channel_from_conwayste.select_next_some() => {
                    if let NetwaysteEvent::GetStatus(ping) = netwayste_request {
                        let server_address = client_state.server_address.unwrap().clone();

                        client_state.latency_filter.start();

                        udp_sink.send((Packet::GetStatus { ping },server_address)).await?;
                    } else {
                        let action: RequestAction = NetwaysteEvent::build_request_action_from_netwayste_event(
                            netwayste_request,
                            client_state.in_game(),
                        );

                        if action != RequestAction::None {
                            match action {
                                RequestAction::Connect { ref name, ..} => {
                                    // TODO: Have the conwayste client provide this
                                    client_state.name = Some(name.to_owned());
                                },
                                _ => {}
                            }

                            let packet = client_state.action_to_packet(action);
                            let server_address = client_state.server_address.unwrap().clone();

                            udp_sink.send((packet, server_address)).await?;
                        }
                    }
                }
            }
        }
    }
}

/*
(conwayste_event) = conwayste_stream.select_next_some() => {
    if let NetwaysteEvent::GetStatus(ping) = netwayste_request {
        let server_address = client_state.server_address.unwrap().clone();

        client_state.latency_filter.start();

        netwayste_send!(
            udp_tx,
            (server_address, Packet::GetStatus { ping }),
            ("Could not send user input cmd to server")
        );
    } else {
        let action: RequestAction = NetwaysteEvent::build_request_action_from_netwayste_event(
            netwayste_request,
            client_state.in_game(),
        );
        match action {
            RequestAction::Connect { ref name, .. } => {
                // TODO: do not store the name on client_state since that's the wrong
                // place for it.
                client_state.name = Some(name.to_owned());
            }
            _ => {}
        }
        client_state.try_server_send(&udp_tx, &exit_tx, action);
    }
}
*/
