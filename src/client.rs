/*
 * A networking library for the multiplayer game, Conwayste.
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

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;

mod net;

use std::env;
use std::io::{self, Read, ErrorKind};
use std::error::Error;
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use net::{RequestAction, ResponseCode, Packet, LineCodec, BroadcastChatMessage, NetworkManager};
use tokio_core::reactor::{Core, Timeout};
use futures::{Future, Sink, Stream, stream};
use futures::future::ok;
use futures::sync::mpsc;

const TICK_INTERVAL:         u64   = 10; // milliseconds
const CLIENT_VERSION: &str = "0.0.1";


struct ClientState {
    sequence:         u64,   // sequence number of requests
    response_ack:     Option<u64>,  // last acknowledged response sequence number from server
    last_req_action:  Option<RequestAction>,   // last one we sent to server TODO: this is wrong;
                                              // assumes network is well-behaved
    name:             Option<String>,
    room:             Option<String>,
    cookie:           Option<String>,
    chat_msg_seq_num: u64,
    tick:             usize,
    network:          NetworkManager,
}

impl ClientState {

    fn new() -> Self {
        ClientState {
            sequence:        0,
            response_ack:    None,
            last_req_action: None,
            name:            None,
            room:            None,
            cookie:          None,
            chat_msg_seq_num: 0,
            tick:            0,
            network:      NetworkManager::new(),
        }
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

    fn handle_response_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr, opt_packet: Option<Packet>) {
        // All `None` packets should get filtered out up the hierarchy
        let packet = opt_packet.unwrap();
        println!("DEBUG: Got packet from server {:?}: {:?}", addr, packet);
        match packet {
            Packet::Response{sequence: _, request_ack: _, code} => {
                // XXX sequence
                // XXX request_ack
                match code {
                    ResponseCode::OK => {
                        match self.handle_response_ok() {
                            Ok(_) => {},
                            Err(e) => println!("{:?}", e)
                        }
                    }
                    ResponseCode::LoggedIn(cookie, server_version) => {
                        self.handle_logged_in(cookie, server_version);
                    }
                    ResponseCode::PlayerList(player_names) => {
                        self.handle_player_list(player_names);
                    }
                    ResponseCode::RoomList(rooms) => {
                        self.handle_room_list(rooms);
                    }
                    ResponseCode::KeepAlive => {},
                    // errors
                    code @ _ => {
                        error!("response from server: {:?}", code);
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
                let send = udp_tx.unbounded_send((addr.clone(), packet));

                if send.is_err() {
                    warn!("Could not send UpdateReply{{ {} }} to server", self.chat_msg_seq_num);
                    self.network.statistics.inc_tx_packets_failed();
                } else {
                    self.network.statistics.inc_tx_packets_success();
                }
            }
            Packet::Request{..} => {
                warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
            }
            Packet::UpdateReply{..} => {
                warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
            }
        }
    }

    fn handle_tick_event(&mut self, udp_tx: &mpsc::UnboundedSender<(SocketAddr, Packet)>, addr: SocketAddr) {
        // Every 100ms, after we've connected
        if self.tick % 100 == 0 && self.cookie.is_some() {
            let keep_alive = Packet::Request {
                sequence: self.sequence,
                response_ack: self.response_ack,
                cookie: self.cookie.clone(),
                action: RequestAction::KeepAlive
            };
            let result = udp_tx.unbounded_send( (addr, keep_alive) );

            if result.is_err() {
                warn!("Could not send KeepAlive");
                self.network.statistics.inc_tx_keep_alive_failed();
            } else {
                self.network.statistics.inc_tx_keep_alive_success();
                info!("Send KeepAlive yay!")
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

                if action == RequestAction::Disconnect {
                    (&exit_tx).unbounded_send(()).unwrap(); // Okay if we panic for FA/we want to quit anyway
                }
            },
        }
        if action != RequestAction::None {
            // Sequence number can increment once we're talking to a server
            if self.cookie != None {
                self.sequence += 1;
            }

            self.last_req_action = Some(action.clone());
            let mut packet = Packet::Request {
                sequence:     self.sequence,
                response_ack: self.response_ack,
                cookie:       self.cookie.clone(),
                action:       action.clone(),
            };

           match action.clone() {
               RequestAction::TestSequenceNumber(b) => {
                   packet = Packet::Request {
                        sequence:     b,
                        response_ack: self.response_ack,
                        cookie:       self.cookie.clone(),
                        action:       action,
                    };
               }
               _ => {}
            }

            self.network.buffer_tx_packet(packet.clone());
            let result = (&udp_tx).unbounded_send((addr.clone(), packet));
            if result.is_err() {
                warn!("Could not send user input cmd to server");
                self.network.statistics.inc_tx_packets_failed();
            } else {
                self.network.statistics.inc_tx_packets_success();
            }
        }
    }

    fn handle_response_ok(&mut self) -> Result<(), Box<Error>> {
        if let Some(ref last_action) = self.last_req_action {
            match last_action {
                &RequestAction::JoinRoom(ref room_name) => {
                    self.room = Some(room_name.clone());
                    println!("Joined room {}.", room_name);
                }
                &RequestAction::LeaveRoom => {
                    if self.in_game() {
                        println!("Left room {}.", self.room.clone().unwrap());
                    }
                    self.room = None;
                    self.chat_msg_seq_num = 0;
                }
                _ => {
                    //XXX more cases in which server replies OK
                    println!("OK :)");
                }
            }
            return Ok(());
        } else {
            //println!("OK, but we didn't make a request :/");
            return Err(Box::new(io::Error::new(ErrorKind::Other, "invalid packet - server-only")));
        }
    }

    fn handle_logged_in(&mut self, cookie: String, server_version: String) {
        self.cookie = Some(cookie);

        println!("Set client name to {:?}", self.name.clone().unwrap());
        self.check_for_upgrade(&server_version);
    }

    fn handle_player_list(&mut self, player_names: Vec<String>) {
        println!("---BEGIN PLAYER LIST---");
        for (i, player_name) in player_names.iter().enumerate() {
            println!("{}\tname: {}", i, player_name);
        }
        println!("---END PLAYER LIST---");
    }

    fn handle_room_list(&mut self, rooms: Vec<(String, u64, bool)>) {
        println!("---BEGIN GAME ROOM LIST---");
        for (game_name, num_players, game_running) in rooms {
            println!("#players: {},\trunning? {:?},\tname: {:?}",
                        num_players,
                        game_running,
                        game_name);
        }
        println!("---END GAME ROOM LIST---");
    }

    fn handle_incoming_chats(&mut self, chats: Option<Vec<BroadcastChatMessage>> ) {
        match chats {
            Some(mut chat_messages) => {
                chat_messages.retain(|ref chat_message| {
                    self.chat_msg_seq_num < chat_message.chat_seq.unwrap()
                });
                // This loop does two things: 1) update chat_msg_seq_num, and
                // 2) prints messages from other players
                for chat_message in chat_messages {
                    let chat_seq = chat_message.chat_seq.unwrap();
                    self.chat_msg_seq_num = std::cmp::max(chat_seq, self.chat_msg_seq_num);

                    match self.name.as_ref() {
                        Some(ref client_name) => {
                            if *client_name != &chat_message.player_name {
                                println!("{}: {}", chat_message.player_name, chat_message.message);
                            }
                        }
                        None => { panic!("Client name not set!"); }
                    }

                }
            }
            None => {}
        }
    }

    fn build_command_request_action(&mut self, cmd: String, args: Vec<String>) -> RequestAction {
        let mut action: RequestAction = RequestAction::None;
        // keep these in sync with print_help function
        match cmd.as_str() {
            "help" => {
                print_help();
            }
            "connect" => {
                if args.len() == 1 {
                    self.name = Some(args[0].clone());
                    action = RequestAction::Connect{
                        name:           args[0].clone(),
                        client_version: CLIENT_VERSION.to_owned(),
                    };
                } else { println!("ERROR: expected client name only"); }
            }
            "disconnect" => {
                if args.len() == 0 {
                    action = RequestAction::Disconnect;
                } else { println!("ERROR: expected no arguments to disconnect"); }
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
                } else { println!("ERROR: expected no arguments to list"); }
            }
            "new" => {
                if args.len() == 1 {
                    action = RequestAction::NewRoom(args[0].clone());
                } else { println!("ERROR: expected name of room only"); }
            }
            "join" => {
                if args.len() == 1 {
                    if !self.in_game() {
                        action = RequestAction::JoinRoom(args[0].clone());
                    } else {
                        println!("ERROR: you are already in a game");
                    }
                } else { println!("ERROR: expected room name only"); }
            }
            "leave" => {
                if args.len() == 0 {
                    if self.in_game() {
                        action = RequestAction::LeaveRoom;
                    } else {
                        println!("ERROR: you are already in the lobby");
                    }
                } else { println!("ERROR: expected no arguments to leave"); }
            }
            "quit" => {
                println!("Peace out!");
                action = RequestAction::Disconnect;
            }
            "sn" => {
                if args.len() != 0 {
                    action = RequestAction::TestSequenceNumber(args[0].parse::<u64>().unwrap());
                }
            }
            _ => {
                println!("ERROR: command not recognized: {}", cmd);
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
    Response((SocketAddr, Option<Packet>)),
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
    drop(env_logger::init());

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:12345".to_owned());
    let addr = addr.parse::<SocketAddr>()
       .unwrap_or_else(|e| {
                    error!("failed to parse address {:?}: {:?}", addr, e);
                    exit(1);
                });
    println!("Connecting to {:?}", addr);

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

    println!("Accepting commands to remote {:?} from local {:?}.\nType /help for more info...", addr, local_addr);

    // initialize state
    let initial_client_state = ClientState::new();

    let iter_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () )); // just a Stream that emits () forever
    // .and_then is like .map except that it processes returned Futures
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(TICK_INTERVAL), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::TickEvent)
        })
    }).map_err(|_| ());

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::Response(packet_tuple)
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

    let main_loop_fut = tick_stream
        .select(packet_stream)
        .select(stdin_stream)
        .fold(initial_client_state, move |mut client_state: ClientState, event| {
            match event {
                Event::Response((addr, opt_packet)) => {
                    client_state.handle_response_event(&udp_tx, addr, opt_packet);
                }
                Event::TickEvent => {
                    client_state.handle_tick_event(&udp_tx, addr);
                }
                Event::UserInputEvent(user_input) => {
                    client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input, addr);
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
        assert!(result.is_err());
    }

    #[test]
    fn handle_response_ok_join_request_sent() {
        let mut client_state = ClientState::new();
        client_state.last_req_action = Some(RequestAction::JoinRoom("some room".to_owned()));
        let result = client_state.handle_response_ok();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn handle_response_ok_leave_request_sent() {
        let mut client_state = ClientState::new();
        client_state.last_req_action = Some(RequestAction::LeaveRoom);
        let result = client_state.handle_response_ok();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn handle_response_ok_none_request_sent() {
        // This tests the `RequestAction::None` case
        let mut client_state = ClientState::new();
        client_state.last_req_action = Some(RequestAction::None);
        let result = client_state.handle_response_ok();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ());
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
            incoming_messages.push( BroadcastChatMessage {
                chat_seq: Some(x as u64),
                player_name: "a player".to_owned(),
                message: format!("message {}", x)
            });
        }

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 10);
    }

    #[test]
    fn handle_incoming_chats_client_is_up_to_date() {
        let mut client_state = ClientState::new();
        client_state.chat_msg_seq_num = 10;

        let incoming_messages = vec![
            BroadcastChatMessage {
                chat_seq: Some(10u64),
                player_name: "a player".to_owned(),
                message: format!("message {}", 10)
            }];

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 10);
    }

    #[test]
    #[should_panic]
    fn handle_incoming_chats_new_messages_player_name_not_set_panics() {
        let mut client_state = ClientState::new();
        client_state.chat_msg_seq_num = 10;

        let incoming_messages = vec![
            BroadcastChatMessage {
                chat_seq: Some(11u64),
                player_name: "a player".to_owned(),
                message: format!("message {}", 11)
            }];

        client_state.handle_incoming_chats(Some(incoming_messages));
    }

    #[test]
    fn handle_incoming_chats_new_messages_are_old_and_new() {
        let mut client_state = ClientState::new();
        client_state.name = Some("client name".to_owned());
        client_state.chat_msg_seq_num = 10;

        let mut incoming_messages = vec![];
        for x in 0..20 {
            incoming_messages.push( BroadcastChatMessage {
                chat_seq: Some(x as u64),
                player_name: "a player".to_owned(),
                message: format!("message {}", x)
            });
        }

        client_state.handle_incoming_chats(Some(incoming_messages));
        assert_eq!(client_state.chat_msg_seq_num, 19);
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

}
