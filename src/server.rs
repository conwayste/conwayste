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

use net::{RequestAction, ResponseCode, Packet, LineCodec, UniUpdateType, BroadcastChatMessage};

use std::error::Error;
use std::io::{self, ErrorKind};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use std::collections::HashMap;
use std::fmt;
use std::time;
use std::collections::VecDeque;
use futures::*;
use futures::future::ok;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Timeout};
use rand::Rng;

const TICK_INTERVAL:         u64   = 40; // milliseconds
const MAX_GAME_SLOT_NAME:    usize = 16;
const MAX_NUM_CHAT_MESSAGES: usize = 128;
const MAX_AGE_CHAT_MESSAGES: usize = 60*5; // seconds

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct PlayerID(u64);

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct GameSlotID(u64);

impl fmt::Display for PlayerID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

impl fmt::Display for GameSlotID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

#[derive(PartialEq, Debug, Clone)]
struct Player {
    player_id:     PlayerID,
    cookie:        String,
    addr:          SocketAddr,
    name:          String,
    request_ack:   Option<u64>,          // most recent request sequence number received
    next_resp_seq: u64,                  // next response sequence number
    game_info:     Option<PlayerInGameInfo>,   // none means in lobby
    last_acked_msg_seq_num: Option<u64> // TODO: move to PlayerInGameInfo
}

// info for a player as it relates to a game/gameslot
#[derive(PartialEq, Debug, Clone)]
struct PlayerInGameInfo {
    game_slot_id: GameSlotID,
    //XXX PlayerGenState ID within Universe
    //XXX update statuses
}

impl Player {
}

#[derive(PartialEq, Debug, Clone)]
struct ServerChatMessage {
    seq_num:     u64,     // sequence number
    player_id:   PlayerID,
    player_name: String,
    message:     String,
    timestamp:   time::Instant,
}

#[derive(Clone, PartialEq)]
struct GameSlot {
    game_slot_id: GameSlotID,
    name:         String,
    player_ids:   Vec<PlayerID>,
    game_running: bool,
    universe:     u64,    // Temp until we integrate
    latest_seq_num: u64,
    messages:     VecDeque<ServerChatMessage>    // Front == Oldest, Back == Newest
}

struct ServerState {
    tick:           u64,
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
*      1) Current timestamp (lower 24 bits)
*      2) A random salt
*
*       64 bits total
*  _________________________
*  |  32 bits  |  32 bits  |
*  | timestamp | rand_salt |
*  |___________|___________|
*/
fn calculate_hash() -> u64 {
        let hash: u64;

        let mut timestamp: u64 = time::Instant::now().elapsed().as_secs().into();
        timestamp = timestamp & 0xFFFFFFFF;

        let mut rand_salt: u64 = rand::thread_rng().next_u32().into();
        rand_salt = rand_salt & 0xFFFFFFFF;

        hash = (timestamp << 32) | rand_salt;
        hash
}

impl GameSlot {
    fn new(name: String, player_ids: Vec<PlayerID>) -> Self {
        GameSlot {
            game_slot_id: GameSlotID(calculate_hash()),
            name: name,
            player_ids:   player_ids,
            game_running: false,
            universe:     0,
            messages:     VecDeque::<ServerChatMessage>::with_capacity(MAX_NUM_CHAT_MESSAGES),
            latest_seq_num: 0,
        }
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
                let player: &Player = self.players.get(&player_id).unwrap();
                if player.game_info == None {
                    return ResponseCode::BadRequest(Some("cannot list players because in lobby.".to_owned()));
                };
                let game_slot_id = &player.game_info.as_ref().unwrap().game_slot_id;  // unwrap ok because of test above
                let gs = self.game_slots.get(&game_slot_id).unwrap();
                // can we use filter AMEEN
                for ref p in self.players.values() {
                    if gs.player_ids.contains(&p.player_id) {
                        players.push(p.name.clone());
                    }
                }
                return ResponseCode::PlayerList(players);
            },
            RequestAction::ChatMessage(msg)  => {
                let player = self.players.get(&player_id);
                let player_in_game = player.is_some() && player.unwrap().game_info.is_some();

                if !player_in_game {
                    return ResponseCode::BadRequest(Some(format!("Player {} has not joined a game.", player_id)))
                }

                // User is in game, Server needs to broadcast this to GameSlot
                let player = player.unwrap();
                let slot_id = player.clone().game_info.unwrap().game_slot_id;

                let opt_slot = self.game_slots.get_mut(&slot_id);

                if opt_slot == None {
                    return ResponseCode::BadRequest(Some(format!("No game found or player {} and slot id {}", player_id, slot_id)))
                }

                let gs = opt_slot.unwrap();
                let ref mut messages: VecDeque<ServerChatMessage> = gs.messages;
                let queue_size = messages.len();
                gs.latest_seq_num += 1;

                if queue_size >= MAX_NUM_CHAT_MESSAGES {
                    for _ in 0..(queue_size-MAX_NUM_CHAT_MESSAGES+1) {
                        messages.pop_front();
                    }
                }

                messages.push_back(ServerChatMessage {
                    player_id: player_id,
                    player_name: player.name.clone(),
                    message: msg,
                    timestamp: time::Instant::now(),
                    seq_num: gs.latest_seq_num
                });
                return ResponseCode::OK
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

                let player: &Player = self.players.get(&player_id).unwrap();
                if player.game_info.is_some() {
                    return ResponseCode::BadRequest(Some("cannot create game slot because in-game.".to_owned()));
                }

                // XXX check name uniqueness
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
                let player: &mut Player = self.players.get_mut(&player_id).unwrap();
                if player.game_info.is_some() {
                    return ResponseCode::BadRequest(Some("cannot join game because already in-game.".to_owned()));
                }
                for ref mut gs in self.game_slots.values_mut() {
                    if gs.name == slot_name {
                        gs.player_ids.push(player.player_id);
                        // TODO: send event to in-game state machine
                        player.game_info = Some(PlayerInGameInfo{ game_slot_id: gs.game_slot_id.clone() });
                        return ResponseCode::OK;
                    }
                }
                return ResponseCode::BadRequest(Some(format!("no game slot named {:?}", slot_name)));
            }
            RequestAction::LeaveGameSlot   => {
                // TODO: DRY up code duplication with other branches of this match (especially ListPlayers)
                // remove current player from its game
                let player: &mut Player = self.players.get_mut(&player_id).unwrap();
                if player.game_info == None {
                    return ResponseCode::BadRequest(Some("cannot leave game because in lobby.".to_owned()));
                };
                {
                    let game_slot_id = &player.game_info.as_ref().unwrap().game_slot_id;  // unwrap ok because of test above
                    for ref mut gs in self.game_slots.values_mut() {
                        if gs.game_slot_id == *game_slot_id {
                            // remove player_id from game slot's player_ids
                            gs.player_ids.retain(|&p_id| p_id != player.player_id);
                            break;
                        }
                    }
                }
                player.game_info = None;
                // TODO: send event to in-game state machine
                return ResponseCode::OK;
            }
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

    // always returns either Ok(Some(Packet::Response{...})), Ok(None), or error
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
                            code:        ResponseCode::LoggedIn(cookie, net::VERSION.to_owned()),
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
            Packet::UpdateReply{cookie, last_chat_seq, last_game_update_seq, last_gen} => {
                let opt_player_id = self.get_player_id_by_cookie(cookie.as_str());
                
                if opt_player_id == None {
                    return Err(Box::new(io::Error::new(ErrorKind::PermissionDenied, "invalid cookie")));
                }

                let player_id = opt_player_id.unwrap();
                let opt_player = self.players.get_mut(&player_id);

                if opt_player == None {
                    return Err(Box::new(io::Error::new(ErrorKind::NotFound, "player not found")));
                }

                let player: &mut Player = opt_player.unwrap();
                if player.last_acked_msg_seq_num.is_none() || player.last_acked_msg_seq_num < last_chat_seq {
                    player.last_acked_msg_seq_num = last_chat_seq;
                }
                Ok(None)
            }

        }
    }

    // XXX
    // Right now we'll be constructing all client Update packets for _every_ game slot.
    fn construct_client_updates(&mut self) -> Result<Option<Vec<(SocketAddr, Packet)>>, Box<Error>> {
        let mut client_updates: Vec<(SocketAddr, Packet)> = vec![];

        if self.game_slots.len() == 0 {
            return Ok(None);
        }

        // For each game slot, determine if each player has unread messages based on last_acked_msg_seq_num
        // TODO: POOR PERFORMANCE BOUNTY
        for slot in self.game_slots.values() {

            if slot.messages.is_empty() || slot.player_ids.len() == 0 {
                continue;
            }

            for player_id in &slot.player_ids {
                let opt_player = self.players.get(&player_id);
                if opt_player.is_none() { continue; }
                let player = opt_player.unwrap();
                // Only send what a player has not yet seen
                let raw_unsent_messages = match player.last_acked_msg_seq_num {
                    Some(last_acked_msg_seq_num) => {

                        let newest_msg = slot.messages.back().unwrap(); // XXX unwrap()'s okay because we know it's non-empty
                        // Player is caught up
                        if last_acked_msg_seq_num == newest_msg.seq_num {
                            continue;
                        } else if last_acked_msg_seq_num > newest_msg.seq_num {
                            println!("ERROR: misbehaving client {}; client says it has more messages than we sent!", player.name);
                            continue;
                        }

                        let oldest_msg = slot.messages.front().unwrap(); // XXX unwrap()'s okay because we know it's non-empty
                        // Skip over these messages since we've already acked them
                        let amount_to_consume: u64 =
                            if last_acked_msg_seq_num >= oldest_msg.seq_num {
                                ((last_acked_msg_seq_num - oldest_msg.seq_num) + 1) % (MAX_NUM_CHAT_MESSAGES as u64)
                            } else if last_acked_msg_seq_num < oldest_msg.seq_num && oldest_msg.seq_num != newest_msg.seq_num {
                                // Sequence number has wrapped
                                (<u64>::max_value() - oldest_msg.seq_num) + last_acked_msg_seq_num + 1
                            } else {
                                0
                            };

                        // Cast to usize is safe because our message containers are limited by MAX_NUM_CHAT_MESSAGES
                        let mut message_iter = slot.messages.iter();
                        message_iter.skip(amount_to_consume as usize).cloned().collect()
                    }
                    None => {
                        // Smithers, unleash the hounds!
                        slot.messages.clone()
                    }
                };

                let unsent_messages: Vec<BroadcastChatMessage> = raw_unsent_messages.iter().map(|msg| {
                    BroadcastChatMessage {
                        chat_seq:    Some(msg.seq_num),
                        player_name: msg.player_name.clone(),
                        message:     msg.message.clone()
                    }
                }).collect();

                let messages_available = !unsent_messages.is_empty();
                let update_packet = Packet::Update {
                    chats:           if !messages_available {None} else {Some(unsent_messages)},
                    game_updates:    None,
                    universe_update: UniUpdateType::NoChange,
                };

                // TODO Add condition for game_updates
                if messages_available {
                    client_updates.push((player.addr.clone(), update_packet));
                }
            }
        }

        // TODO Universe updates are TBD
        if client_updates.len() > 0 {
            info!("Updates available");
            Ok(Some(client_updates))
        }
        else {
            Ok(None)
        }
    }

    fn expire_old_messages(&mut self) {
        if self.game_slots.len() != 0 {
            let current_timestamp = time::Instant::now();
            for slot in self.game_slots.values_mut() {
                if !slot.messages.is_empty() {
                    slot.messages.retain(|ref m| current_timestamp - m.timestamp < Duration::from_secs(MAX_AGE_CHAT_MESSAGES as u64) );
                }
            }
        }
    }

    fn new_player(&mut self, name: String, addr: SocketAddr) -> Player {
        let cookie = new_cookie();
        Player {
            player_id:     PlayerID(calculate_hash()),
            cookie:        cookie,
            addr:          addr,
            name:   name,
            request_ack:   None,
            next_resp_seq: 0,
            game_info:     None,
            last_acked_msg_seq_num: None
        }
    }

    fn new() -> Self {
        ServerState {
            tick:              0,
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
        .fold(initial_server_state, move |mut server_state, event| {
            match event {
                Event::Request(packet_tuple) => {
                     // With the above filter, `packet` should never be None
                    let (addr, opt_packet) = packet_tuple;
                    println!("got {:?} and {:?}!", addr, opt_packet);

                    // Decode incoming and send a Response to the Requester
                    if let Some(packet) = opt_packet {
                        let decode_result = server_state.decode_packet(addr, packet.clone());
                        if decode_result.is_ok() {
                            let opt_response_packet = decode_result.unwrap();

                            if let Some(response_packet) = opt_response_packet {
                                let response = (addr.clone(), response_packet);
                                (&tx).unbounded_send(response).unwrap();
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

                    server_state.expire_old_messages();
                    let client_update_packets_result = server_state.construct_client_updates();
                    if client_update_packets_result.is_ok() {
                        let opt_update_packets = client_update_packets_result.unwrap();

                        if let Some(update_packets) = opt_update_packets {
                            for update in update_packets {
                                tx.unbounded_send(update).unwrap();
                            }
                        }
                    }
                }
            }

            // return the updated client for the next iteration
            ok(server_state)
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

