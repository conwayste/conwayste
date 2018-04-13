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

use std::error::Error;
use std::io::{self, ErrorKind};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use std::collections::HashMap;
use std::fmt;
use std::time;
use futures::*;
use futures::future::ok;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Timeout};
use rand::Rng;

const TICK_INTERVAL:      u64   = 40; // milliseconds
const MAX_GAME_SLOT_NAME: usize = 16;

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct PlayerID(u64);

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct GameSlotID(u64);

impl fmt::Display for PlayerID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

#[derive(PartialEq, Debug, Clone)]
struct Player {
    player_id:     PlayerID,
    cookie:        String,
    addr:          SocketAddr,
    name:   String,
    request_ack:   Option<u64>,          // most recent request sequence number received
    next_resp_seq: u64,                  // next response sequence number
    game_info:     Option<PlayerInGameInfo>,   // none means in lobby
}

// info for a player as it relates to a game/gameslot
#[derive(PartialEq, Debug, Clone)]
struct PlayerInGameInfo {
    game_slot_id: GameSlotID,
    //XXX PlayerGenState ID within Universe
    //XXX update statuses
}

impl Player {
    fn calc_id(&mut self) {
        let player_id = calculate_hash_from_name(&self.name);
        self.player_id = PlayerID(player_id);
    }
}

#[derive(Clone)]
struct GameSlot {
    game_slot_id: GameSlotID,
    name:         String,
    player_ids:   Vec<PlayerID>,
    game_running: bool,
    universe:     u64,    // Temp until we integrate
    pending_messages: Vec<(PlayerID, String)>
}

struct ServerState {
    tick:           u64,
    ctr:            u64,
    players:        HashMap<PlayerID, Player>,
    player_map:     HashMap<String, PlayerID>,      // map cookie to player ID
    game_slots:     HashMap<GameSlotID, GameSlot>,
    game_slot_map:  HashMap<String, GameSlotID>,      // map slot name to slot ID
}

//////////////// Utilities ///////////////////////

fn new_cookie() -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::encode(&buf)
}

/*
*  Entity (Player/GameSlot) IDs are comprised of:
*      1) The entity name XOR'd into itself for each character
*      2) Current timestamp (lower 24 bits)
*      3) A salt
*
*              64 bits total
*   _____________________________________
*  |    8 bits   |  24 bits  |  32 bits  |
*  | entity_name | timestamp | rand_salt |
*  |_____________|___________|___________|
*/
fn calculate_hash_from_name(name: &String) -> u64 {
        let hash: u64;
        let mut name_byte_accumulator: u64 = 0;
        let name_as_bytes = name.clone().into_bytes();

        for character in name_as_bytes {
            name_byte_accumulator ^= character as u64;
        }

        let mut timestamp: u64 = time::Instant::now().elapsed().as_secs().into();
        timestamp = timestamp & 0xFFFFFF;

        let mut rand_salt: u64 = rand::thread_rng().next_u32().into();
        rand_salt = rand_salt & 0xFFFFFFFFFFFFFF;

        hash = rand_salt | (timestamp << 32) | (name_byte_accumulator << 56);
        hash
}

impl GameSlot {
    fn new(name: String, player_ids: Vec<PlayerID>) -> Self {
        let mut slot = GameSlot {
            game_slot_id: GameSlotID(0),   // TODO: better unique ID generation
            name,
            player_ids:   player_ids,
            game_running: false,
            universe:     0,
            pending_messages: vec![]
        };
        slot.calc_id();
        slot
    }

    fn calc_id(&mut self) {
        let slot_id = calculate_hash_from_name(&self.name);
        self.game_slot_id = GameSlotID(slot_id);
    }
}

impl ServerState {
    // not used for connect
    fn process_request_action(&mut self, player_id: PlayerID, action: RequestAction) -> ResponseCode {
        match action {
            RequestAction::Disconnect      => unimplemented!(),
            RequestAction::KeepAlive       => unimplemented!(),
            RequestAction::ListPlayers     => {
                let mut players = vec![];
                for ref p in self.players.values() {
                    players.push(p.name.clone());
                }
                ResponseCode::PlayerList(players)
            },
            RequestAction::ChatMessage(msg)  => {
                let player_info = self.players.get(&player_id);
                let player_in_game = player_info.is_some() && player_info.unwrap().game_info.is_some();

                match player_in_game {
                    true => {
                        // User is in game, Server needs to broadcast this to GameSlot
                        let slot_id = player_info.unwrap().clone().game_info.unwrap().game_slot_id;

                        let opt_slot = self.game_slots.get_mut(&slot_id);
                        match opt_slot {
                            Some(gs) => {
                                let ref mut deliver : Vec<(PlayerID,String)> = gs.pending_messages;
                                deliver.push((player_id, msg));
                                ResponseCode::OK
                            }
                            None => {
                                ResponseCode::BadRequest(Some(format!("Player \"{}\" not in game", player_id)))
                            }
                        }
/*                        let mut found_slot = false;
                        for gs in &mut self.game_slots {
                            if gs.game_slot_id == slot_id {
                                let ref mut deliver : Vec<(PlayerID,String)> = gs.pending_messages;
                                deliver.push((player_id, msg));
                                found_slot = true;
                                break;
                            }
                        }
                        match found_slot {
                            true => ResponseCode::OK,
                            false => ResponseCode::BadRequest(Some(format!("Player \"{}\" not in game", player_id))),
                        }
*/
                    }
                    false => {
                        ResponseCode::BadRequest(Some(format!("Player \"{}\" not found", player_id)))
                    }
                }
            },
            RequestAction::ListGameSlots   => {
                let mut slots = vec![];
                for ref gs in self.game_slots.values() {
                    slots.push((gs.name.clone(), gs.player_ids.len() as u64, gs.game_running));
                }
                ResponseCode::GameSlotList(slots)
            }
            RequestAction::NewGameSlot(name)  => {
                // validate length
                if name.len() > MAX_GAME_SLOT_NAME {
                    return ResponseCode::BadRequest(Some(format!("game slot name too long; max {} characters",
                                                                 MAX_GAME_SLOT_NAME)));
                }
                // create game slot
                if !self.game_slot_map.get(&name).is_some() {
                    let game_slot = GameSlot::new(name.clone(), vec![]);

                    self.game_slot_map.insert(name, game_slot.game_slot_id);
                    self.game_slots.insert(game_slot.game_slot_id, game_slot);

                    ResponseCode::OK
                } else {
                    ResponseCode::BadRequest(Some(format!("game slot name already in use")))
                }
            }
            RequestAction::JoinGameSlot(slot_name) => {
                let player = self.players.get_mut(&player_id).unwrap();
                let opt_slot_id = self.game_slot_map.get(&slot_name);
                match opt_slot_id {
                    Some(slot_id) => {
                        let opt_game_slot = self.game_slots.get_mut(&slot_id);
                        match opt_game_slot {
                            Some(gs) => {
                                gs.player_ids.push(player.player_id);

                                player.game_info = Some(PlayerInGameInfo {
                                    game_slot_id: gs.clone().game_slot_id
                                });
                                // TODO: send event to in-game state machine
                                ResponseCode::OK
                            }
                            None => {
                                ResponseCode::BadRequest(Some(format!("no game slot found for ID {:?}", slot_id)))
                            }
                        }
                    }
                    None => {
                        ResponseCode::BadRequest(Some(format!("no game slot named {:?}", slot_name)))
                    }
                }
            }
            RequestAction::LeaveGameSlot   => unimplemented!(),
            RequestAction::Connect{..}     => panic!(),
            RequestAction::None            => panic!(),
        }
    }

    fn is_unique_name(&self, name: &str) -> bool {
        for ref player in self.players.values() {
            if player.name == name {
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

                        // save player into players hash map, and save player ID into hash map using cookie
                        self.player_map.insert(cookie.clone(), player.player_id);
                        self.players.insert(player.player_id, player);

                        let response = Packet::Response{
                            sequence:    sequence,
                            request_ack: None,
                            code:        ResponseCode::LoggedIn(cookie),
                        };
                        return Ok(Some(response));
                    } else {
                        // not a unique name
                        let response = Packet::Response{
                            sequence:    0,
                            request_ack: None,
                            code:        ResponseCode::Unauthorized(Some("not a unique name".to_owned())),
                        };
                        return Ok(Some(response));
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
                    match action {
                        RequestAction::Connect{..} => unreachable!(),
                        _ => {
                            let response_code = self.process_request_action(player_id, action);
                            let sequence = {
                                let player: &mut Player = self.players.get_mut(&player_id).unwrap();
                                let sequence = player.next_resp_seq;
                                player.next_resp_seq += 1;
                                sequence
                            };
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
                    assert_eq!(true, player.addr != addr && player.name != name);
                });

                self.players.push(Player::new(name, addr));
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
/*
    fn initiate_player_session(&mut self) {
        //XXX
        if self.has_pending_players() {
            if let Some(mut a) = self.players.pop() {
                if let Some(mut b) = self.players.pop() {
                    let game_slot = GameSlot::new("some game slot".to_owned(), vec![a.player_id, b.player_id]);
                    a.game_info = Some(PlayerInGameInfo{ game_slot_id: game_slot.game_slot_id.clone() });
                    b.game_info = Some(PlayerInGameInfo{ game_slot_id: game_slot.game_slot_id.clone() });
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
*/
    fn new_player(&mut self, name: String, addr: SocketAddr) -> Player {
        let cookie = new_cookie();
        let mut player = Player {
            player_id:     PlayerID(0),
            cookie:        cookie,
            addr:          addr,
            name:   name,
            request_ack:   None,
            next_resp_seq: 0,
            game_info:     None,
        };

        player.calc_id();
        player
    }

    fn new() -> Self {
        ServerState {
            tick:              0,
            ctr:               0,
            players:           HashMap::<PlayerID, Player>::new(),
            game_slots:        HashMap::<GameSlotID, GameSlot>::new(),
            player_map:        HashMap::<String, PlayerID>::new(),
            game_slot_map:     HashMap::<String, GameSlotID>::new(),
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
                                    player_a.name, player_a.player_id,
                                    player_b.name, player_b.player_id,
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

