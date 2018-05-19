#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;

mod net;

use std::env;
use std::io::{self, Read, Error};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use net::{RequestAction, ResponseCode, Packet, LineCodec};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::{Core, Timeout};
use futures::{Future, Sink, Stream, stream};
use futures::future::ok;
use futures::sync::mpsc;

const CLIENT_VERSION: &str = "0.0.1";

struct ClientState {
    sequence:     u64,   // sequence number of requests
    response_ack: Option<u64>,  // last acknowledged response sequence number from server
    last_req_action: Option<RequestAction>,   // last one we sent to server TODO: this is wrong;
                                              // assumes network is well-behaved
    name:         Option<String>,
    room:    Option<String>,
    cookie:       Option<String>,
    chat_msg_seq_num: u64,
}

impl ClientState {
    fn in_game(&self) -> bool {
        self.room.is_some()
    }

    fn check_for_upgrade(&self, server_version: &String) {
        let client_version = &net::VERSION.to_owned();
        if client_version < server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nnWarning: Client out-of-date. Please upgrade.", client_version, server_version)
        }
        else if client_version > server_version {
            warn!("\tClient Version: {}\n\tServer Version: {}\nWarning: Client Version greater than Server Version.", client_version, server_version)
        }
    }
}

//////////////// Event Handling /////////////////
enum UserInput {
    Command{cmd: String, args: Vec<String>},   // command with arguments
    Chat(String),
}

enum Event {
    TickEvent,   // note: currently unused
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

    // Bind to a UDP socket
    let addr_to_bind = if addr.ip().is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    let udp = UdpSocket::bind(&addr_to_bind, &handle)
        .expect("failed to bind socket");
    let local_addr = udp.local_addr().unwrap();

    // Channels
    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();
    let (udp_tx, udp_rx) = mpsc::unbounded();    // create a channel because we can't pass the sink around everywhere
    let (exit_tx, exit_rx) = mpsc::unbounded();  // send () to exit_tx channel to quit the client

    println!("Accepting commands to remote {:?} from local {:?}.\nType /help for more info...", addr, local_addr);

    // initialize state
    let initial_client_state = ClientState {
        sequence:     0,
        response_ack: None,
        last_req_action: None,
        name:         None,
        room:    None,
        cookie:       None,      // not connected yet
        chat_msg_seq_num: 0,
    };

    let iter_stream = stream::iter_ok::<_, Error>(iter::repeat( () )); // just a Stream that emits () forever
    // .and_then is like .map except that it processes returned Futures
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(1000), &handle).unwrap();
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
                    let packet = opt_packet.unwrap();
                    println!("DEBUG: Got packet from server {:?}: {:?}", addr, packet);
                    match packet {
                        Packet::Response{sequence, request_ack, code} => {
                            // XXX sequence
                            // XXX request_ack
                            match code {
                                ResponseCode::OK => {
                                    if let Some(ref last_action) = client_state.last_req_action {
                                        match last_action {
                                            &RequestAction::JoinRoom(ref room_name) => {
                                                client_state.room = Some(room_name.clone());
                                                println!("Joined room {}.", room_name);
                                            }
                                            &RequestAction::LeaveRoom => {
                                                if client_state.in_game() {
                                                    println!("Left room {}.", client_state.room.unwrap());
                                                }
                                                client_state.room = None;
                                            }
                                            _ => {
                                                //XXX more cases in which server replies OK
                                                println!("OK :)");
                                            }
                                        }
                                    } else {
                                        println!("OK, but we didn't make a request :/");
                                    }
                                }
                                ResponseCode::LoggedIn(cookie, server_version) => {
                                    client_state.cookie = Some(cookie);
                                    println!("Now logged into server.");

                                    client_state.check_for_upgrade(&server_version);
                                }
                                ResponseCode::PlayerList(player_names) => {
                                    println!("---BEGIN PLAYER LIST---");
                                    let mut n = 1;
                                    for player_name in player_names {
                                        println!("{}\tname: {}", n, player_name);
                                        n += 1;
                                    }
                                    println!("---END PLAYER LIST---");
                                }
                                ResponseCode::RoomList(rooms) => {
                                    println!("---BEGIN GAME ROOM LIST---");
                                    for (game_name, num_players, game_running) in rooms {
                                        println!("#players: {},\trunning? {:?},\tname: {:?}",
                                                 num_players,
                                                 game_running,
                                                 game_name);
                                    }
                                    println!("---END GAME ROOM LIST---");
                                }
                                // errors
                                code @ _ => {
                                    error!("response from server: {:?}", code);
                                }
                            }
                        }
                        Packet::Update{chats, game_updates, universe_update} => {
                            match chats {
                                Some(mut chat_messages) => {
                                    chat_messages.retain(|ref chat_message| {
                                        client_state.chat_msg_seq_num < chat_message.chat_seq.unwrap()
                                    });
                                    // This loop does two things: 1) update chat_msg_seq_num, and
                                    // 2) prints messages from other players
                                    for chat_message in chat_messages {
                                        let chat_seq = chat_message.chat_seq.unwrap();
                                        client_state.chat_msg_seq_num = std::cmp::max(chat_seq, client_state.chat_msg_seq_num);
                                        if client_state.name.as_ref().unwrap() != &chat_message.player_name {
                                            println!("{}: {}", chat_message.player_name, chat_message.message);
                                        }
                                    }
                                }
                                None => {}
                            }
                            // TODO game updates

                            // Reply to the update
                            let packet = Packet::UpdateReply{
                                cookie:               client_state.cookie.clone().unwrap(),
                                last_chat_seq:        Some(client_state.chat_msg_seq_num),
                                last_game_update_seq: None,
                                last_gen:             None,
                            };
                            let _ = udp_tx.unbounded_send((addr.clone(), packet));
                        }
                        Packet::Request{..} => {
                            warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
                        }
                        Packet::UpdateReply{..} => {
                            warn!("Ignoring packet from server normally sent by clients: {:?}", packet);
                        }
                    }
                }
                Event::TickEvent => {
                    /*
                    // send a packet with ctr
                    let act = if client_state.ctr == 0 {
                         RequestAction::Connect
                    }
                    else {
                        RequestAction::None
                    };
                    let packet = Packet {
                        player_name: "Joe".to_owned(),
                        number:      client_state.ctr,
                        action:      act
                    };
                    // send packet
                    let _ = udp_tx.unbounded_send((addr.clone(), packet));
                    println!("Sent a packet! ctr is {}", client_state.ctr);

                    // just for fun, change our client state
                    client_state.ctr += 1;
                    */
                }
                Event::UserInputEvent(user_input) => {
                    let mut action = RequestAction::None;
                    match user_input {
                        UserInput::Chat(string) => {
                            action = RequestAction::ChatMessage(string);
                        }
                        UserInput::Command{cmd, args} => {
                            // keep these in sync with print_help function
                            match cmd.as_str() {
                                "help" => {
                                    print_help();
                                }
                                "connect" => {
                                    if args.len() == 1 {
                                        client_state.name = Some(args[0].clone());
                                        println!("Set client name to {:?}", client_state.name.clone().unwrap());
                                        action = RequestAction::Connect{
                                            name:           args[0].clone(),
                                            client_version: CLIENT_VERSION.to_owned(),
                                        };
                                    } else { println!("ERROR: expected one argument to connect"); }
                                }
                                "disconnect" => {
                                    if args.len() == 0 {
                                        action = RequestAction::Disconnect;
                                    } else { println!("ERROR: expected no arguments to disconnect"); }
                                }
                                "list" => {
                                    if args.len() == 0 {
                                        // players or rooms
                                        if client_state.in_game() {
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
                                    } else { println!("ERROR: expected one argument to new"); }
                                }
                                "join" => {
                                    if args.len() == 1 {
                                        if !client_state.in_game() {
                                            action = RequestAction::JoinRoom(args[0].clone());
                                        } else {
                                            println!("ERROR: you are already in a game");
                                        }
                                    } else { println!("ERROR: expected one argument to join"); }
                                }
                                "leave" => {
                                    if args.len() == 0 {
                                        if client_state.in_game() {
                                            action = RequestAction::LeaveRoom;
                                        } else {
                                            println!("ERROR: you are already in the lobby");
                                        }
                                    } else { println!("ERROR: expected no arguments to leave"); }
                                }
                                "quit" => {
                                    println!("Peace out!");
                                    (&exit_tx).unbounded_send(()).unwrap();
                                }
                                _ => {
                                    println!("ERROR: command not recognized: {}", cmd);
                                }
                            }
                        },
                    }
                    if action != RequestAction::None {
                        client_state.last_req_action = Some(action.clone());
                        let packet = Packet::Request {
                            sequence:     client_state.sequence,
                            response_ack: client_state.response_ack,
                            cookie:       client_state.cookie.clone(),
                            action:       action,
                        };
                        let _ = (&udp_tx).unbounded_send((addr.clone(), packet));
                        client_state.sequence += 1;
                    }
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
