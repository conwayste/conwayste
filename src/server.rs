#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;
extern crate base64;
extern crate rand;

mod net;

use net::{RequestAction, ResponseCode, Packet, LineCodec};
/*
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
*/
use std::error::Error;
use std::io::{self, ErrorKind};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use std::collections::HashMap;
use futures::*;
use futures::future::ok;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Timeout};
use rand::Rng;

const TICK_INTERVAL: u64 = 40; // milliseconds

#[derive(PartialEq, Debug, Clone, Copy)]
struct PlayerID(usize);

#[derive(PartialEq, Debug, Clone)]
struct Player {
    player_id:     PlayerID,
    cookie:        String,
    addr:          SocketAddr,
    player_name:   String,
    request_ack:   Option<u64>,          // most recent request sequence number received
    next_resp_seq: u64,                  // next response sequence number
    game:          Option<GamePlayer>,   // none means in lobby
}

// info for a player as it relates to a game/gameslot
#[derive(PartialEq, Debug, Clone)]
struct GamePlayer {
    game_slot_id: u64,
    //XXX PlayerGenState ID within Universe
    //XXX update statuses
}

impl Player {
    /*
    fn new(name: String, addr: SocketAddr) -> Self {
        let id = calculate_hash(&PlayerID {name: name.clone(), addr: addr});
        Player {
            player_name: name,
            player_id: id,
            addr: addr,
            in_game: false,
        }
    }
    */
}

/*
impl Hash for PlayerID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.addr.hash(state);
    }
}
*/

struct GameSlot {
    game_slot_id: u64,
    player_ids:   Vec<PlayerID>,
    game_running: bool,
    universe:     u64,    // Temp until we integrate
}

struct ServerState {
    tick:           u64,
    ctr:            u64,
    players:        Vec<Player>,
    player_map:     HashMap<String, PlayerID>,      // map cookie to player ID
    game_slots:     Vec<GameSlot>,
    next_player_id: PlayerID,  // index into players
    next_game_slot_id: u64,
}

//////////////// Utilities ///////////////////////

/*
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
*/

fn new_cookie() -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::encode(&buf)
}

impl ServerState {
    // not used for connect
    fn process_request_action(&mut self, action: RequestAction) -> ResponseCode {
        match action {
            RequestAction::Disconnect      => unimplemented!(),
            RequestAction::KeepAlive       => unimplemented!(),
            RequestAction::ListPlayers     => unimplemented!(),
            RequestAction::ChatMessage(_)  => unimplemented!(),
            RequestAction::ListGameSlots   => unimplemented!(),
            RequestAction::NewGameSlot(_)  => unimplemented!(),
            RequestAction::JoinGameSlot(_) => unimplemented!(),
            RequestAction::LeaveGameSlot   => unimplemented!(),
            RequestAction::Connect{..}     => panic!(),
            RequestAction::None            => panic!(),
        }
    }

    fn is_unique_name(&self, name: &str) -> bool {
        for ref player in self.players.iter() {
            if player.player_name == name {
                return false;
            }
        }
        true
    }

    fn get_player_id_by_cookie(&self, cookie: &str) -> Option<PlayerID> {
        match self.player_map.get(cookie) {
            Some(player_id) => Some(*player_id),
            None => None
        }
    }

    // always returs either Ok(Some(Packet::Response{...})), Ok(None), or error
    fn decode_packet(&mut self, addr: SocketAddr, packet: Packet) -> Result<Option<Packet>, Box<Error>> {
        match packet {
            pkt @ Packet::Response{..} | pkt @ Packet::Update{..} => {
                return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "invalid packet - server-only")));
            }
            Packet::Request{sequence, response_ack, cookie, action} => {
                match action {
                    RequestAction::Connect{..} => (),
                    _ => {
                        if cookie == None {
                            return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "no cookie")));
                        }
                    }
                }

                // handle connect (create user, and save cookie)
                if let RequestAction::Connect{name, client_version} = action {
                    if self.is_unique_name(&name) {
                        let mut player = self.new_player(name.clone(), addr.clone());
                        let cookie = player.cookie.clone();
                        let sequence = player.next_resp_seq;
                        player.next_resp_seq += 1;

                        // save player into players vec, and save player ID into hash map using cookie
                        self.player_map.insert(cookie.clone(), player.player_id);
                        self.players.push(player);

                        let response = Packet::Response{
                            sequence:    sequence,
                            request_ack: None,
                            code:        ResponseCode::LoggedIn(cookie),
                        };
                        return Ok(Some(response));
                    } else {
                        return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "not a unique name")));
                    }
                } else {
                    // look up player by cookie
                    let cookie = match cookie {
                        Some(cookie) => cookie,
                        None => {
                            return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "cookie required for non-connect actions")));
                        }
                    };
                    let player_id = match self.get_player_id_by_cookie(cookie.as_str()) {
                        Some(player_id) => player_id,
                        None => {
                            return Err(Box::new(io::Error::new(ErrorKind::PermissionDenied, "invalid cookie")));
                        }
                    };
                    //XXX same thing as get_player_id_by_cookie
                    let player: &mut Player = self.players.get_mut(player_id.0).unwrap();
                    match action {
                        RequestAction::Connect{..} => unreachable!(),
                        _ => {
                            let response_code = self.process_request_action(action);
                            let sequence = player.next_resp_seq;
                            player.next_resp_seq += 1;
                            let response = Packet::Response{
                                sequence:    sequence,
                                request_ack: None,
                                code:        response_code,
                            };
                            Ok(Some(response))
                        }
                    }
                }
            }
            Packet::UpdateReply{..} => {
                unimplemented!();
            }
        }
        /*
        match action {
            RequestAction::Connect => {
                self.players.iter().for_each(|player| {
                    assert_eq!(true, player.addr != addr && player.player_name != player_name);
                });

                self.players.push(Player::new(player_name, addr));
            },
            RequestAction::Ack                 => {},
            RequestAction::Disconnect          => {},
            RequestAction::JoinGame            => {},
            RequestAction::ListPlayers         => {},
            RequestAction::ChatMessage(String) => {},
            RequestAction::None                => {},
        }
        */
    }

    fn has_pending_players(&self) -> bool {
        !self.players.is_empty() && self.players.len() % 2 == 0
    }

    fn initiate_player_session(&mut self) {
        //XXX
        if self.has_pending_players() {
            if let Some(mut a) = self.players.pop() {
                if let Some(mut b) = self.players.pop() {
                    let game_slot = self.new_game_slot(vec![a.player_id, b.player_id]);
                    a.game = Some(GamePlayer{ game_slot_id: game_slot.game_slot_id });
                    b.game = Some(GamePlayer{ game_slot_id: game_slot.game_slot_id });
                    self.game_slots.push(game_slot);
                    self.ctr+=1;
                }
                else {
                    panic!("Unavailable player B");
                }
            }
            else {
                panic!("Unavailable player A");
            }
        }
    }

    fn new_player(&mut self, name: String, addr: SocketAddr) -> Player {
        let id = self.next_player_id;
        self.next_player_id = PlayerID(id.0 + 1);
        let cookie = new_cookie();
        Player {
            player_id:     id,
            cookie:        cookie,
            addr:          addr,
            player_name:   name,
            request_ack:   None,
            next_resp_seq: 0,
            game:          None,
        }
    }

    fn new_game_slot(&mut self, player_ids: Vec<PlayerID>) -> GameSlot {
        let id = self.next_game_slot_id;
        self.next_game_slot_id += 1;
        GameSlot {
            game_slot_id: id,
            player_ids:   player_ids,
            game_running: false,
            universe:     0,
        }
    }

    fn new() -> Self {
        ServerState {
            tick:              0,
            ctr:               0,
            players:           Vec::<Player>::new(),
            game_slots:        Vec::<GameSlot>::new(),
            player_map:        HashMap::<String, PlayerID>::new(),
            next_player_id:    PlayerID(0),
            next_game_slot_id: 0,
        }
    }
}

//////////////// Event Handling /////////////////
enum Event {
    TickEvent,
    Request((SocketAddr, Option<Packet>)),
//    Notify((SocketAddr, Option<Packet>)),
}

//////////////////// Main /////////////////////
fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (tx, rx) = mpsc::unbounded();

    let udp = net::bind(&handle, None, None)
        .unwrap_or_else(|e| {
            error!("Error while trying to bind UDP socket: {:?}", e);
            exit(1);
        });

    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();

    let initial_server_state = ServerState::new();

    let iter_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
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
            Event::Request(packet_tuple)
        })
        .map_err(|_| ());

    let server_fut = tick_stream
        .select(packet_stream)
        .fold((tx.clone(), initial_server_state), move |(tx, mut server_state), event| {
            match event {
                Event::Request(packet_tuple) => {
                     // With the above filter, `packet` should never be None
                    let (addr, opt_packet) = packet_tuple;
                    println!("got {:?} and {:?}!", addr, opt_packet);

                    if let Some(packet) = opt_packet {
                        let decode_result = server_state.decode_packet(addr, packet.clone());
                        if decode_result.is_ok() {
                            let opt_response_packet = decode_result.unwrap();
                            //XXX send packet
                            if let Some(response_packet) = opt_response_packet {
                                let response = (addr.clone(), response_packet);
                                tx.unbounded_send(response).unwrap();
                            }
                        } else {
                            let err = decode_result.unwrap_err();
                            println!("ERROR decoding packet from {:?}: {}", addr, err.description());
                        }
                    }
                }

                Event::TickEvent => {
                    // Server tick
                    // Likely spawn off work to handle server tasks here
                    server_state.tick += 1;

                    /*
                    server_state.initiate_player_session();

                    if server_state.ctr == 1 {
                        // GameSlot tick
                        server_state.game_slots.iter()
                            .filter(|ref conn| conn.player_a.in_game && conn.player_b.in_game)
                            .for_each(|ref conn| {
                                let player_a = &conn.player_a;
                                let player_b = &conn.player_b;
                                let uni = &conn.universe;
                                println!("Session: {}({:x}) versus {}({:x}), generation: {}",
                                    player_a.player_name, player_a.player_id,
                                    player_b.player_name, player_b.player_id,
                                    uni);
                            });

                        server_state.ctr += 1;
                    }
                    */
                }
            }

            // return the updated client for the next iteration
            ok((tx, server_state))
        })
        .map(|_| ())
        .map_err(|_| ());

    let sink_fut = rx.fold(udp_sink, |udp_sink, outgoing_item| {
            let udp_sink = udp_sink.send(outgoing_item).map_err(|_| ());    // this method flushes (if too slow, use send_all)
            udp_sink
        }).map(|_| ()).map_err(|_| ());

    let combined_fut = server_fut.map(|_| ())
        .select(sink_fut)
        .map(|_| ());   // wait for either server_fut or sink_fut to complete

    drop(core.run(combined_fut));
}

