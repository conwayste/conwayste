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

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;
extern crate chrono;

mod net;
use std::env;
use std::io::{self, Read, Write};
use std::error::Error;
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};
use crate::net::{
    RequestAction, ResponseCode, Packet, LineCodec,
    BroadcastChatMessage, NetworkManager, NetworkQueue,
};
use tokio_core::reactor::{Core, Timeout};
use futures::{Future, Sink, Stream, stream, future::ok, sync::mpsc};
use log::LevelFilter;
use chrono::Local;

const TICK_INTERVAL_IN_MS:          u64    = 1000;
const NETWORK_INTERVAL_IN_MS:       u64    = 1000;
const CLIENT_VERSION: &str = "0.0.1";


struct ClientState {
    sequence:         u64,          // Sequence number of requests
    response_sequence: u64,         // Value of the next expected sequence number from the server,
                                    // and indicates the sequence number of the next process-able rx packet
    name:             Option<String>,
    room:             Option<String>,
    cookie:           Option<String>,
    chat_msg_seq_num: u64,
    tick:             usize,
    network:          NetworkManager,
    heartbeat:        Option<Instant>,
    disconnect_initiated: bool,
}

impl ClientState {

    fn new() -> Self {
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

    fn reset(&mut self) {
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

    fn in_game(&self) -> bool {
        self.room.is_some()
    }

    // XXX Once netwayste integration is complete, we'll need to internally send
    // the result of most of these handlers so we can notify a player via UI event.

    fn check_for_upgrade(&self, server_version: &String) {
        let client_version = &net::VERSION.to_owned();
        if client_version < server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nnWarning: Client out-of-date. Please upgrade.", client_version, server_version);
        }
        else if client_version > server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nWarning: Client Version greater than Server Version.", client_version, server_version);
        }
    }

    fn process_queued_server_responses(&mut self) {
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

    fn process_event_code(&mut self, code: ResponseCode) {
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

    fn handle_incoming_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr, opt_packet: Option<Packet>) {
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

    fn handle_network_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr) {
        if self.cookie.is_some() {
            // Determine what can be processed
            // Determine what needs to be resent
            // Resend anything remaining in TX queue if it has also expired.
            self.process_queued_server_responses();

            let indices = self.network.tx_packets.get_retransmit_indices();

            self.network.retransmit_expired_tx_packets(udp_tx, addr, Some(self.response_sequence), &indices);
        }
    }

    fn handle_tick_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr) {
        // Every 100ms, after we've connected
        if self.cookie.is_some() {
            // Send a keep alive heartbeat if the connection is live
            let keep_alive = Packet::Request {
                cookie: self.cookie.clone(),
                sequence: self.sequence,
                response_ack: None,
                action: RequestAction::KeepAlive(self.response_sequence),
            };
            let timed_out = net::has_connection_timed_out(self.heartbeat);

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

    fn handle_user_input_event(&mut self,
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

    fn handle_response_ok(&mut self) -> Result<(), Box<Error>> {
            info!("OK :)");
            return Ok(());
    }

    fn handle_logged_in(&mut self, cookie: String, server_version: String) {
        self.cookie = Some(cookie);

        info!("Set client name to {:?}", self.name.clone().unwrap());
        self.check_for_upgrade(&server_version);
    }

    fn handle_joined_room(&mut self, room_name: &String) {
            self.room = Some(room_name.clone());
            info!("Joined room: {}", room_name);
    }

    fn handle_left_room(&mut self) {
        if self.in_game() {
            info!("Left room {}.", self.room.clone().unwrap());
        }
        self.room = None;
        self.chat_msg_seq_num = 0;
    }

    fn handle_player_list(&mut self, player_names: Vec<String>) {
        info!("---BEGIN PLAYER LIST---");
        for (i, player_name) in player_names.iter().enumerate() {
            info!("{}\tname: {}", i, player_name);
        }
        info!("---END PLAYER LIST---");
    }

    fn handle_room_list(&mut self, rooms: Vec<(String, u64, bool)>) {
        info!("---BEGIN GAME ROOM LIST---");
        for (game_name, num_players, game_running) in rooms {
            info!("#players: {},\trunning? {:?},\tname: {:?}",
                        num_players,
                        game_running,
                        game_name);
        }
        info!("---END GAME ROOM LIST---");
    }

    fn handle_incoming_chats(&mut self, chats: Option<Vec<BroadcastChatMessage>> ) {
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

    fn build_command_request_action(&mut self, cmd: String, args: Vec<String>) -> RequestAction {
        let mut action: RequestAction = RequestAction::None;
        // keep these in sync with print_help function
        match cmd.as_str() {
            "help" => {
                print_help();
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
            "leave" => {
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

//////////////// Event Handling /////////////////
#[derive(PartialEq, Debug, Clone)]
enum UserInput {
    Command{cmd: String, args: Vec<String>},
    Chat(String),
}

enum Event {
    TickEvent,
    UserInputEvent(UserInput),
    Incoming((SocketAddr, Option<Packet>)),
    NetworkEvent,
//    NotifyAck((SocketAddr, Option<Packet>)),
}

////////////////// Utilities //////////////////
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

//////////////////// Main /////////////////////
fn main() {
    env_logger::Builder::new()
    .format(|buf, record| {
        writeln!(buf,
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

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:12345".to_owned());
    let addr = addr.parse::<SocketAddr>()
       .unwrap_or_else(|e| {
                    error!("failed to parse address {:?}: {:?}", addr, e);
                    exit(1);
                });
    trace!("Connecting to {:?}", addr);

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Have separate thread read from stdin
    let (stdin_tx, stdin_rx) = mpsc::unbounded::<Vec<u8>>();
    let stdin_rx = stdin_rx.map_err(|_| panic!()); // errors not possible on rx

    // Unwrap ok because bind will abort if unsuccessful
    let udp = net::bind(&handle, Some("0.0.0.0"), Some(0)).unwrap();
    let local_addr = udp.local_addr().unwrap();

    // Channels
    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();
    let (udp_tx, udp_rx) = mpsc::unbounded();    // create a channel because we can't pass the sink around everywhere
    let (exit_tx, exit_rx) = mpsc::unbounded();  // send () to exit_tx channel to quit the client

    trace!("Accepting commands to remote {:?} from local {:?}.", addr, local_addr);
    trace!("Type /help for more info...");

    // initialize state
    let initial_client_state = ClientState::new();

    let iter_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () )); // just a Stream that emits () forever
    // .and_then is like .map except that it processes returned Futures
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(TICK_INTERVAL_IN_MS), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::TickEvent)
        })
    }).map_err(|_| ());

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::Incoming(packet_tuple)
        })
        .map_err(|_| ());

    let stdin_stream = stdin_rx
        .map(|buf| {
            let string = String::from_utf8(buf).unwrap();
            let string = String::from_str(string.trim()).unwrap();
            if !string.is_empty() && string != "" {
                Some(string)
            } else {
                None        // empty line; will be filtered out in next step
            }
        })
        .filter(|opt_string| {
            *opt_string != None
        })
        .map(|opt_string| {
            let string = opt_string.unwrap();
            let user_input = parse_stdin(string);
            Event::UserInputEvent(user_input)
        }).map_err(|_| ());

    let network_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
    let network_stream = network_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(NETWORK_INTERVAL_IN_MS), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::NetworkEvent)
        })
    }).map_err(|_| ());

    let main_loop_fut = tick_stream
        .select(packet_stream)
        .select(stdin_stream)
        .select(network_stream)
        .fold(initial_client_state, move |mut client_state: ClientState, event| {
            match event {
                Event::Incoming((addr, opt_packet)) => {
                    client_state.handle_incoming_event(&udp_tx, addr, opt_packet);
                }
                Event::TickEvent => {
                    client_state.handle_tick_event(&udp_tx, addr);
                }
                Event::UserInputEvent(user_input) => {
                    client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input, addr);
                }
                Event::NetworkEvent => {
                    client_state.handle_network_event(&udp_tx, addr);
                }
            }

            // finally, return the updated client state for the next iteration
            ok(client_state)
        })
        .map(|_| ())
        .map_err(|_| ());

    // listen on the channel created above and send it to the UDP sink
    let sink_fut = udp_rx.fold(udp_sink, |udp_sink, outgoing_item| {
        udp_sink.send(outgoing_item).map_err(|_| ())    // this method flushes (if too slow, use send_all)
    }).map(|_| ()).map_err(|_| ());

    let exit_fut = exit_rx.into_future().map(|_| ()).map_err(|_| ());

    let combined_fut = exit_fut
                        .select(main_loop_fut).map(|_| ()).map_err(|_| ())
                        .select(sink_fut).map_err(|_| ());

    thread::spawn(move || {
        read_stdin(stdin_tx);
    });
    drop(core.run(combined_fut).unwrap());
}

// Our helper method which will read data from stdin and send it along the
// sender provided. This is blocking so should be on separate thread.
fn read_stdin(mut tx: mpsc::UnboundedSender<Vec<u8>>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) |
            Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx = match tx.send(buf).wait() {
            Ok(tx) => tx,
            Err(_) => break,
        };
    }
}

// At this point we should only have command or chat message to work with
fn parse_stdin(mut input: String) -> UserInput {
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

#[cfg(test)]
mod test {
    use super::*;

    fn fake_socket_addr() -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5678)
    }

    #[test]
    fn handle_response_ok_no_request_sent() {
        let mut client_state = ClientState::new();
        let result = client_state.handle_response_ok();
        assert!(result.is_ok());
    }

    #[test]
    fn handle_logged_in_verify_connection_cookie() {
        let mut client_state = ClientState::new();
        client_state.name = Some("Dr. Cookie Monster, Esquire".to_owned());
        assert_eq!(client_state.cookie, None);
        client_state.handle_logged_in("cookie monster".to_owned(), CLIENT_VERSION.to_owned());
        assert_eq!(client_state.cookie, Some("cookie monster".to_owned()));
    }

    #[test]
    fn handle_incoming_chats_no_new_chat_messages() {
        let mut client_state = ClientState::new();
        assert_eq!(client_state.chat_msg_seq_num, 0);

        client_state.handle_incoming_chats(None);
        assert_eq!(client_state.chat_msg_seq_num, 0);
    }

    #[test]
    fn handle_incoming_chats_new_messages_are_older() {
        let mut client_state = ClientState::new();
        client_state.chat_msg_seq_num = 10;

        let mut incoming_messages = vec![];
        for x in 0..10 {
            let new_msg =  BroadcastChatMessage::new(x as u64, "a player".to_owned(), format!("message {}", x));
            incoming_messages.push(new_msg);
        }

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 10);
    }

    #[test]
    fn handle_incoming_chats_client_is_up_to_date() {
        let mut client_state = ClientState::new();
        client_state.chat_msg_seq_num = 10;

        let incoming_messages = vec![ BroadcastChatMessage::new(10u64, "a player".to_owned(), format!("message {}", 10))];

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 10);
    }

    #[test]
    #[should_panic]
    fn handle_incoming_chats_new_messages_player_name_not_set_panics() {
        let mut client_state = ClientState::new();
        client_state.chat_msg_seq_num = 10;

        let incoming_messages = vec![ BroadcastChatMessage::new(11u64, "a player".to_owned(), format!("message {}", 11))];

        client_state.handle_incoming_chats(Some(incoming_messages));
    }

    #[test]
    fn handle_incoming_chats_new_messages_are_old_and_new() {
        let mut client_state = ClientState::new();
        let starting_chat_seq_num = 10;
        client_state.name = Some("client name".to_owned());
        client_state.chat_msg_seq_num = starting_chat_seq_num;

        let mut incoming_messages = vec![];
        for x in 0..20 {
            let new_msg =  BroadcastChatMessage::new(x as u64, "a player".to_owned(), format!("message {}", x));
            incoming_messages.push(new_msg);
        }

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 19);

        let mut seq_num = starting_chat_seq_num+1;
        let chat_queue = &client_state.network.rx_chat_messages.as_ref().unwrap().queue;
        for msg in chat_queue {
            assert_eq!(msg.chat_seq.unwrap(), seq_num);
            seq_num+=1;
        }
    }

    #[test]
    fn parse_stdin_input_has_no_leading_forward_slash() {
        let chat = parse_stdin("some text".to_owned());
        assert_eq!(chat, UserInput::Chat("some text".to_owned()));
    }

    #[test]
    fn parse_stdin_input_no_arguments() {
        let cmd = parse_stdin("/helpusobi".to_owned());
        assert_eq!(cmd, UserInput::Command{ cmd: "helpusobi".to_owned(), args: vec![]});

    }

    #[test]
    fn parse_stdin_input_multiple_arguments() {
        let cmd = parse_stdin("/helpusobi 1".to_owned());
        assert_eq!(cmd, UserInput::Command{ cmd: "helpusobi".to_owned(), args: vec!["1".to_owned()]});

        let cmd = parse_stdin("/helpusobi 1 you".to_owned());
        assert_eq!(cmd, UserInput::Command{ cmd: "helpusobi".to_owned(), args: vec!["1".to_owned(), "you".to_owned()]});

        let cmd = parse_stdin("/helpusobi 1 you are our only hope".to_owned());
        assert_eq!(cmd, UserInput::Command{ cmd: "helpusobi".to_owned(), args: vec!["1".to_owned(),
                                                                         "you".to_owned(),
                                                                         "are".to_owned(),
                                                                         "our".to_owned(),
                                                                         "only".to_owned(),
                                                                         "hope".to_owned()
                                                                         ]});
    }

    #[test]
    fn build_command_request_action_unknown_command() {
        let command = UserInput::Command{ cmd: "helpusobi".to_owned(), args: vec!["1".to_owned()]};

        let mut client_state = ClientState::new();
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_help_returns_no_action() {
        let command = UserInput::Command{ cmd: "help".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_disconnect() {
        let command = UserInput::Command{ cmd: "disconnect".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::Disconnect);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_disconnect_with_args_returns_no_action() {
        let command = UserInput::Command{ cmd: "disconnect".to_owned(), args: vec!["1".to_owned()]};

        let mut client_state = ClientState::new();
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_list_in_lobby() {
        let command = UserInput::Command{ cmd: "list".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::ListRooms);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_list_in_game() {
        let command = UserInput::Command{ cmd: "list".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::ListPlayers);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_leave_cases() {
        let command = UserInput::Command{ cmd: "leave".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        // Not in a room
        match command.clone() {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }

        // Happy to leave
        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::LeaveRoom);
            },
            UserInput::Chat(_) => {unreachable!()},
        }

        // Even though we're in a room, you cannot specify anything else
        let command = UserInput::Command{ cmd: "leave".to_owned(), args: vec!["some room".to_owned()]};
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn build_command_request_action_join_cases() {
        let command = UserInput::Command{ cmd: "join".to_owned(), args: vec![]};

        let mut client_state = ClientState::new();
        // no room specified
        match command.clone() {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }

        // Already in game
        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            },
            UserInput::Chat(_) => {unreachable!()},
        }

        // Happily join one
        client_state.room = None;
        let command = UserInput::Command{ cmd: "join".to_owned(), args: vec!["some room".to_owned()]};
        match command {
            UserInput::Command{cmd, args} => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::JoinRoom("some room".to_owned()));
            },
            UserInput::Chat(_) => {unreachable!()},
        }
    }

    #[test]
    fn handle_user_input_event_increment_sequence_number() {
        // There is a lot that _could_ be tested here but most of it is handled in the above test cases.
        let mut client_state = ClientState::new();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let user_input = UserInput::Chat("memes".to_owned());
        let addr = fake_socket_addr();

        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input, addr.clone());
        assert_eq!(client_state.sequence, 1);

        let user_input = UserInput::Chat("and another one".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input, addr);
        assert_eq!(client_state.sequence, 2);
    }

    #[test]
    fn handle_incoming_event_basic_tx_rx_queueing() {
        let mut client_state = ClientState::new();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let addr = fake_socket_addr();
        let connect_cmd = UserInput::Command{cmd: "connect".to_owned(), args: vec!["name".to_owned()]};
        let new_room_cmd = UserInput::Command{cmd: "new".to_owned(), args: vec!["room_name".to_owned()]};
        let join_room_cmd = UserInput::Command{cmd: "join".to_owned(), args: vec!["room_name".to_owned()]};
        let leave_room_cmd = UserInput::Command{cmd: "leave".to_owned(), args: vec![]};

        client_state.sequence = 0;
        client_state.response_sequence = 1;
        client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd, addr);         // Seq 0
        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        // dequeue connect since we don't actually want to process it later
        client_state.network.tx_packets.clear();
        client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd, addr);        // Seq 1
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd, addr);       // Seq 2
        client_state.room = Some("room_name".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd, addr);      // Seq 3
        assert_eq!(client_state.sequence, 3);
        assert_eq!(client_state.response_sequence, 1);
        assert_eq!(client_state.network.tx_packets.len(), 3);
        assert_eq!(client_state.network.rx_packets.len(), 0);

        let room_response = Packet::Response{sequence: 1, request_ack: Some(1), code: ResponseCode::OK};
        let join_response = Packet::Response{sequence: 2, request_ack: Some(2), code: ResponseCode::OK};
        let leave_response = Packet::Response{sequence:3, request_ack: Some(3), code: ResponseCode::OK};

        client_state.handle_incoming_event(&udp_tx, addr, Some(leave_response));    // 3 arrives
        assert_eq!(client_state.network.tx_packets.len(), 2);
        assert_eq!(client_state.network.rx_packets.len(), 1);
        client_state.handle_incoming_event(&udp_tx, addr, Some(join_response));     // 2 arrives
        assert_eq!(client_state.network.tx_packets.len(), 1);
        assert_eq!(client_state.network.rx_packets.len(), 2);
        client_state.handle_incoming_event(&udp_tx, addr, Some(room_response));     // 1 arrives
        assert_eq!(client_state.network.tx_packets.len(), 0);
        // RX should be cleared out because upon processing packet sequence '1', RX queue will be contiguous
        assert_eq!(client_state.network.rx_packets.len(), 0);
    }

    #[test]
    fn handle_incoming_event_basic_tx_rx_queueing_cannot_process_all_responses() {
        let mut client_state = ClientState::new();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let addr = fake_socket_addr();
        let connect_cmd = UserInput::Command{cmd: "connect".to_owned(), args: vec!["name".to_owned()]};
        let new_room_cmd = UserInput::Command{cmd: "new".to_owned(), args: vec!["room_name".to_owned()]};
        let join_room_cmd = UserInput::Command{cmd: "join".to_owned(), args: vec!["room_name".to_owned()]};
        let leave_room_cmd = UserInput::Command{cmd: "leave".to_owned(), args: vec![]};

        client_state.sequence = 0;
        client_state.response_sequence = 1;
        client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd, addr);         // Seq 0
        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        // dequeue connect since we don't actually want to process it later
        client_state.network.tx_packets.clear();
        client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd, addr);          // Seq 1
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd.clone(), addr); // Seq 2
        client_state.room = Some("room_name".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd, addr);        // Seq 3
        client_state.room = None; // Temporarily set to None so we can process the next join
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd, addr);         // Seq 4
        client_state.room = Some("room_name".to_owned());
        assert_eq!(client_state.sequence, 4);
        assert_eq!(client_state.response_sequence, 1);
        assert_eq!(client_state.network.tx_packets.len(), 4);
        assert_eq!(client_state.network.rx_packets.len(), 0);

        let room_response = Packet::Response{sequence: 1, request_ack: Some(1), code: ResponseCode::OK};
        let join_response = Packet::Response{sequence: 2, request_ack: Some(2), code: ResponseCode::OK};
        let _leave_response = Packet::Response{sequence: 3, request_ack: Some(3), code: ResponseCode::OK};
        let join2_response = Packet::Response{sequence: 4, request_ack: Some(4), code: ResponseCode::OK};

        // The intent is that 3 never arrives
        client_state.handle_incoming_event(&udp_tx, addr, Some(join2_response));    // 4 arrives
        assert_eq!(client_state.network.tx_packets.len(), 3);
        assert_eq!(client_state.network.rx_packets.len(), 1);
        client_state.handle_incoming_event(&udp_tx, addr, Some(join_response));     // 2 arrives
        assert_eq!(client_state.network.tx_packets.len(), 2);
        assert_eq!(client_state.network.rx_packets.len(), 2);
        client_state.handle_incoming_event(&udp_tx, addr, Some(room_response));     // 1 arrives
        assert_eq!(client_state.network.tx_packets.len(), 1);
        assert_eq!(client_state.network.rx_packets.len(), 1);
    }

    #[test]
    fn handle_incoming_event_basic_tx_rx_queueing_arrives_at_server_out_of_order() {
        let mut client_state = ClientState::new();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let addr = fake_socket_addr();
        let connect_cmd = UserInput::Command{cmd: "connect".to_owned(), args: vec!["name".to_owned()]};
        let new_room_cmd = UserInput::Command{cmd: "new".to_owned(), args: vec!["room_name".to_owned()]};
        let join_room_cmd = UserInput::Command{cmd: "join".to_owned(), args: vec!["room_name".to_owned()]};
        let leave_room_cmd = UserInput::Command{cmd: "leave".to_owned(), args: vec![]};

        client_state.sequence = 0;
        client_state.response_sequence = 1;
        client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd, addr);         // Seq 0
        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        // dequeue connect since we don't actually want to process it later
        client_state.network.tx_packets.clear();
        client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd, addr);        // Seq 1
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd, addr);       // Seq 2
        client_state.room = Some("room_name".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd, addr);      // Seq 3
        assert_eq!(client_state.sequence, 3);
        assert_eq!(client_state.response_sequence, 1);
        assert_eq!(client_state.network.tx_packets.len(), 3);
        assert_eq!(client_state.network.rx_packets.len(), 0);

        // An out-of-order arrival at the server means the response packet's sequence number will not be 1:1 mapping
        // as in the first basic tested above. End result should be the same in both cases.
        let room_response = Packet::Response{sequence: 2, request_ack: Some(1), code: ResponseCode::OK};
        let join_response = Packet::Response{sequence: 3, request_ack: Some(2), code: ResponseCode::OK};
        let leave_response = Packet::Response{sequence:1, request_ack: Some(3), code: ResponseCode::OK};

        client_state.handle_incoming_event(&udp_tx, addr, Some(leave_response));    // client 3 arrives, can process
        assert_eq!(client_state.network.tx_packets.len(), 2);
        assert_eq!(client_state.network.rx_packets.len(), 0);
        client_state.handle_incoming_event(&udp_tx, addr, Some(join_response));     // client 2 arrives, cannot process
        assert_eq!(client_state.network.tx_packets.len(), 1);
        assert_eq!(client_state.network.rx_packets.len(), 1);
        client_state.handle_incoming_event(&udp_tx, addr, Some(room_response));     // client 1 arrives, can process all
        assert_eq!(client_state.network.tx_packets.len(), 0);
        assert_eq!(client_state.network.rx_packets.len(), 0);
    }
}
