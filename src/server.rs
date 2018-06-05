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
const MAX_ROOM_NAME:    usize = 16;
const MAX_NUM_CHAT_MESSAGES: usize = 128;
const MAX_AGE_CHAT_MESSAGES: usize = 60*5; // seconds

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct PlayerID(u64);

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
struct RoomID(u64);

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
struct Player {
    player_id:     PlayerID,
    cookie:        String,
    addr:          SocketAddr,
    name:          String,
    request_ack:   Option<u64>,          // most recent request sequence number received
    next_resp_seq: u64,                  // This is the sequence number for the most recent Response packet the Server sent to the Client
    game_info:     Option<PlayerInGameInfo>,   // none means in lobby
    last_acked_msg_seq_num: Option<u64> // TODO: move to PlayerInGameInfo
}

// info for a player as it relates to a game/room
#[derive(PartialEq, Debug, Clone)]
struct PlayerInGameInfo {
    room_id: RoomID,
    //XXX PlayerGenState ID within Universe
    //XXX update statuses
}

impl Player {
    fn increment_response_seq_num(&mut self) -> u64 {
        let old_seq = self.next_resp_seq;
        self.next_resp_seq += 1;
        old_seq
    }
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
struct Room {
    room_id: RoomID,
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
    rooms:          HashMap<RoomID, Room>,
    room_map:       HashMap<String, RoomID>,      // map room name to room ID
}

//////////////// Utilities ///////////////////////

fn new_cookie() -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::encode(&buf)
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
fn new_uuid() -> u64 {
    let hash: u64;

    let mut timestamp: u64 = time::Instant::now().elapsed().as_secs().into();
    timestamp = timestamp & 0xFFFFFFFF;

    let mut rand_salt: u64 = rand::thread_rng().next_u32().into();
    rand_salt = rand_salt & 0xFFFFFFFF;

    hash = (timestamp << 32) | rand_salt;
    hash
}

fn validate_client_version(_client_version: String) -> bool {
    //TODO: Implement via TDD
    true
}

impl ServerChatMessage {
    fn new(id: PlayerID, name: String, msg: String, seq_num: u64) -> Self {
        ServerChatMessage {
            player_id: id,
            player_name: name,
            message: msg,
            seq_num: seq_num,
            timestamp: time::Instant::now()
        }
    }
}

impl Room {
    fn new(name: String, player_ids: Vec<PlayerID>) -> Self {
        Room {
            room_id: RoomID(new_uuid()),
            name: name,
            player_ids:   player_ids,
            game_running: false,
            universe:     0,
            messages:     VecDeque::<ServerChatMessage>::with_capacity(MAX_NUM_CHAT_MESSAGES),
            latest_seq_num: 0,
        }
    }

    fn discard_older_messages(&mut self) {
        let queue_size = self.messages.len();
        if queue_size >= MAX_NUM_CHAT_MESSAGES {
            for _ in 0..(queue_size-MAX_NUM_CHAT_MESSAGES+1) {
                self.messages.pop_front();
            }
        }
    }

    fn increment_seq_num(&mut self) -> u64 {
        self.latest_seq_num += 1;
        self.latest_seq_num
    }

    fn add_message(&mut self, new_message: ServerChatMessage) {
        self.messages.push_back(new_message);
    }
}

impl ServerState {

    fn get_player(&self, player_id: PlayerID) -> &Player {
        let opt_player = self.players.get(&player_id);

        if opt_player.is_none() {
            panic!("player_id: {} could not be found!");
        }

        opt_player.unwrap()
    }

    fn get_room_id(&self, player_id: PlayerID) -> Option<RoomID> {
        let player = self.get_player(player_id);
        if player.game_info == None {
            return None;
        };

        Some(player.game_info.as_ref().unwrap().room_id)  // unwrap ok because of test above
    }

    fn get_room_mut(&mut self, player_id: PlayerID) -> Option<&mut Room> {
        let opt_room_id = self.get_room_id(player_id);

        if opt_room_id.is_none() {
            return None;
        }
        self.rooms.get_mut(&opt_room_id.unwrap())
    }

    fn get_room(&self, player_id: PlayerID) -> Option<&Room> {
        let opt_room_id = self.get_room_id(player_id);

        if opt_room_id.is_none() {
            return None;
        }
        self.rooms.get(&opt_room_id.unwrap())
    }

    fn list_players(&self, player_id: PlayerID) -> ResponseCode {
        let opt_room = self.get_room(player_id);
        if opt_room.is_none() {
            return ResponseCode::BadRequest(Some("cannot list players because in lobby.".to_owned()));
        }
        let room = opt_room.unwrap();

        let mut players = vec![];
        self.players.values().for_each(|p| {
            if room.player_ids.contains(&p.player_id) {
                players.push(p.name.clone());
            }
        });

        return ResponseCode::PlayerList(players);
    }

    fn handle_chat_message(&mut self, player_id: PlayerID, msg: String) -> ResponseCode {
        let player_in_game = self.is_player_in_game(player_id);

        if !player_in_game {
            return ResponseCode::BadRequest(Some(format!("Player {} has not joined a game.", player_id)));
        }

        // We're borrowing self mutably below, so let's grab this now
        let player_name = {
            let player = self.players.get(&player_id);
            player.unwrap().name.clone()
        };

        // User is in game, Server needs to broadcast this to Room
        let opt_room = self.get_room_mut(player_id);

        if opt_room.is_none() {
            return ResponseCode::BadRequest(Some( format!("Player \"{}\" should be in a room! None found.", player_id )));
        }

        let room = opt_room.unwrap();
        let seq_num = room.increment_seq_num();

        room.discard_older_messages();
        room.add_message(ServerChatMessage::new(player_id, player_name, msg, seq_num));

        return ResponseCode::OK;
    }

    fn list_rooms(&mut self) -> ResponseCode {
        let mut rooms = vec![];
        self.rooms.values().for_each(|gs| {
            rooms.push((gs.name.clone(), gs.player_ids.len() as u64, gs.game_running));
        });
        ResponseCode::RoomList(rooms)
    }

    fn new_room(&mut self, name: String) {
        let room = Room::new(name.clone(), vec![]);

        self.room_map.insert(name, room.room_id);
        self.rooms.insert(room.room_id, room);
    }

    fn create_new_room(&mut self, player_id: PlayerID, room_name: String) -> ResponseCode {
        // validate length
        if room_name.len() > MAX_ROOM_NAME {
            return ResponseCode::BadRequest(Some(format!("room name too long; max {} characters",
                                                            MAX_ROOM_NAME)));
        }

        if self.is_player_in_game(player_id) {
            return ResponseCode::BadRequest(Some("cannot create room because in-game.".to_owned()));
        }

        // Create room if the room name is not already taken
        if !self.room_map.get(&room_name).is_some() {
            self.new_room(room_name.clone());

            return ResponseCode::OK;
        } else {
            return ResponseCode::BadRequest(Some(format!("room name already in use")));
        }
    }

    fn join_room(&mut self, player_id: PlayerID, room_name: String) -> ResponseCode {
        let already_playing = self.is_player_in_game(player_id);
        if already_playing {
            return ResponseCode::BadRequest(Some("cannot join game because already in-game.".to_owned()));
        }

        let player: &mut Player = self.players.get_mut(&player_id).unwrap();

        // TODO replace loop with `get_key_value` once it reaches stable. Same thing with `leave_room` algorithm
        for ref mut gs in self.rooms.values_mut() {
            if gs.name == room_name {
                gs.player_ids.push(player_id);
                player.game_info = Some(PlayerInGameInfo{ room_id: gs.room_id.clone() });
                return ResponseCode::OK;
            }
        }
        return ResponseCode::BadRequest(Some(format!("no room named {:?}", room_name)));
    }

    fn leave_room(&mut self, player_id: PlayerID) -> ResponseCode {
        let already_playing = self.is_player_in_game(player_id);
        if !already_playing {
            return ResponseCode::BadRequest(Some("cannot leave game because in lobby.".to_owned()));
        }
        
        let player: &mut Player = self.players.get_mut(&player_id).unwrap();
        {
            let room_id = &player.game_info.as_ref().unwrap().room_id;  // unwrap ok because of test above
            for ref mut gs in self.rooms.values_mut() {
                if gs.room_id == *room_id {
                    // remove player_id from room's player_ids
                    gs.player_ids.retain(|&p_id| p_id != player.player_id);
                    break;
                }
            }
        }
        player.game_info = None;

        return ResponseCode::OK;
    }

    // not used for connect
    fn process_request_action(&mut self, player_id: PlayerID, action: RequestAction) -> ResponseCode {
        match action {
            RequestAction::Disconnect      => unimplemented!(),
            RequestAction::KeepAlive       => unimplemented!(),
            RequestAction::ListPlayers     => {
                return self.list_players(player_id);
            },
            RequestAction::ChatMessage(msg)  => {
                return self.handle_chat_message(player_id, msg);
            },
            RequestAction::ListRooms   => {
                return self.list_rooms();
            }
            RequestAction::NewRoom(name)  => {
                return self.create_new_room(player_id, name);
            }
            RequestAction::JoinRoom(room_name) => {
                return self.join_room(player_id, room_name);
            }
            RequestAction::LeaveRoom   => {
                return self.leave_room(player_id);
            }
            RequestAction::Connect{..}     => panic!(),
            RequestAction::None            => panic!(),
        }
    }

    fn is_player_in_game(&self, player_id: PlayerID) -> bool {
        let player: Option<&Player> = self.players.get(&player_id);
        player.is_some() && player.unwrap().game_info.is_some()
    }

    fn is_unique_player_name(&self, name: &str) -> bool {
        for ref player in self.players.values() {
            if player.name == name {
                return false;
            }
        }
        return true;
    }

    fn add_new_player(&mut self, name: String, addr: SocketAddr) -> String {
        let mut player = self.new_player(name, addr);

        let player_id = player.player_id;
        let cookie = player.cookie.clone();

        let _ = player.increment_response_seq_num();

        // save player into players hash map, and save player ID into hash map using cookie
        self.player_map.insert(cookie.clone(), player_id);
        self.players.insert(player_id, player);
        cookie
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
            _pkt @ Packet::Response{..} | _pkt @ Packet::Update{..} => {
                return Err(Box::new(io::Error::new(ErrorKind::InvalidData, "invalid packet - server-only")));
            }
            Packet::Request{sequence: _, response_ack: _, cookie, action} => {
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
                    if validate_client_version(client_version) {
                        let response = self.handle_new_connection(name, addr);
                        return Ok(Some(response));
                    } else {
                        return Err(Box::new(io::Error::new(ErrorKind::Other, "client out of date -- please upgrade")));
                    };
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
                            let response = self.handle_request_action(player_id, action);
                            Ok(Some(response))
                        }
                    }
                }
            }
            Packet::UpdateReply{cookie, last_chat_seq, last_game_update_seq: _, last_gen: _} => {
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

    fn handle_request_action(&mut self, player_id: PlayerID, action: RequestAction) -> Packet {
        let response_code = self.process_request_action(player_id, action);

        let sequence = {
            let player: &mut Player = self.players.get_mut(&player_id).unwrap();
            let sequence = player.increment_response_seq_num();
            sequence
        };

        Packet::Response{
            sequence:    sequence,
            request_ack: None,
            code:        response_code,
        }
    }

    fn handle_new_connection(&mut self, name: String, addr: SocketAddr) -> Packet {
        if self.is_unique_player_name(&name) {
            let cookie = self.add_new_player(name.clone(), addr.clone());

            let response = Packet::Response{
                sequence:    0,
                request_ack: None,
                code:        ResponseCode::LoggedIn(cookie, net::VERSION.to_owned()),
            };
            return response;
        } else {
            // not a unique name
            let response = Packet::Response{
                sequence:    0,
                request_ack: None,
                code:        ResponseCode::Unauthorized(Some("not a unique name".to_owned())),
            };
            return response;
        }
    }

    // XXX
    // Right now we'll be constructing all client Update packets for _every_ room.
    fn construct_client_updates(&mut self) -> Result<Option<Vec<(SocketAddr, Packet)>>, Box<Error>> {
        let mut client_updates: Vec<(SocketAddr, Packet)> = vec![];

        if self.rooms.len() == 0 {
            return Ok(None);
        }

        // For each room, determine if each player has unread messages based on last_acked_msg_seq_num
        // TODO: POOR PERFORMANCE BOUNTY
        for room in self.rooms.values() {

            if room.messages.is_empty() || room.player_ids.len() == 0 {
                continue;
            }

            for player_id in &room.player_ids {
                let opt_player = self.players.get(&player_id);
                if opt_player.is_none() { continue; }
                let player = opt_player.unwrap();
                // Only send what a player has not yet seen
                let raw_unsent_messages = match player.last_acked_msg_seq_num {
                    Some(last_acked_msg_seq_num) => {

                        let newest_msg = room.messages.back().unwrap(); // XXX unwrap()'s okay because we know it's non-empty
                        // Player is caught up
                        if last_acked_msg_seq_num == newest_msg.seq_num {
                            continue;
                        } else if last_acked_msg_seq_num > newest_msg.seq_num {
                            println!("ERROR: misbehaving client {}; client says it has more messages than we sent!", player.name);
                            continue;
                        }

                        let oldest_msg = room.messages.front().unwrap(); // XXX unwrap()'s okay because we know it's non-empty
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
                        let mut message_iter = room.messages.iter();
                        message_iter.skip(amount_to_consume as usize).cloned().collect()
                    }
                    None => {
                        // Smithers, unleash the hounds!
                        room.messages.clone()
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

                if messages_available {
                    client_updates.push((player.addr.clone(), update_packet));
                }
            }
        }

        if client_updates.len() > 0 {
            Ok(Some(client_updates))
        }
        else {
            Ok(None)
        }
    }

    fn expire_old_messages_in_all_rooms(&mut self) {
        if self.rooms.len() != 0 {
            let current_timestamp = time::Instant::now();
            for room in self.rooms.values_mut() {
                if !room.messages.is_empty() {
                    room.messages.retain(|ref m| current_timestamp - m.timestamp < Duration::from_secs(MAX_AGE_CHAT_MESSAGES as u64) );
                }
            }
        }
    }

    fn new_player(&mut self, name: String, addr: SocketAddr) -> Player {
        let cookie = new_cookie();
        Player {
            player_id:     PlayerID(new_uuid()),
            cookie:        cookie,
            addr:          addr,
            name:          name,
            request_ack:   None,
            next_resp_seq: 0,
            game_info:     None,
            last_acked_msg_seq_num: None
        }
    }

    fn new() -> Self {
        ServerState {
            tick:       0,
            players:    HashMap::<PlayerID, Player>::new(),
            rooms:      HashMap::<RoomID, Room>::new(),
            player_map: HashMap::<String, PlayerID>::new(),
            room_map:   HashMap::<String, RoomID>::new(),
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

                    server_state.expire_old_messages_in_all_rooms();
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

