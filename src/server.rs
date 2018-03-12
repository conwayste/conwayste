#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;
extern crate base64;

mod net;

use net::{RequestAction, Packet, LineCodec};
/*
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
*/
use std::io::{Error};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use futures::*;
use futures::future::ok;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Timeout};
use base64;
use rand;

const TICK_INTERVAL: u64 = 40; // milliseconds

#[derive(PartialEq, Debug, Clone, Copy)]
struct PlayerID(usize);

#[derive(PartialEq, Debug, Clone)]
struct Player {
    player_id:    PlayerID,
    player_name:  String,
    cookie:       String,
    addr:         SocketAddr,
    game_slot_id: Option<usize>,   // None means in lobby
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
    player_ids:   Vec<PlayerID>,
    game_running: bool,
    universe:     u64,    // Temp until we integrate
}

impl GameSlot {
    fn new(player_ids: Vec<PlayerID>) -> Self {
        GameSlot {
            player_ids:   player_ids,
            game_running: false,
            universe:     0,
        }
    }
}

struct ServerState {
    tick:           u64,
    ctr:            u64,
    players:        Vec<Player>,
    game_slots:     Vec<GameSlot>,
    next_player_id: PlayerID(usize),  // index into players
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
    fn decode_packet(&mut self, addr: SocketAddr, packet: Packet) {
        let player_name = packet.player_name;
        let action = packet.action;

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
    }

    fn has_pending_players(&self) -> bool {
        !self.players.is_empty() && self.players.len() % 2 == 0
    }

    fn initiate_player_session(&mut self) {
        if self.has_pending_players() {
            if let Some(mut a) = self.players.pop() {
                if let Some(mut b) = self.players.pop() {
                    a.in_game = true;
                    b.in_game = true;
                    self.game_slots.push(GameSlot::new(a, b));
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

    fn new_player(&mut self, name: String, addr, SocketAddr) -> Player {
        let id = self.next_player_id;
        self.next_player_id = PlayerID(id.0 + 1);
        let cookie = new_cookie();
        Player {
            player_name: name,
            cookie:      cookie,
            player_id:   id,
            addr:        addr,
            in_game:     false,
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

    let initial_server_state = ServerState { 
        tick: 0,
        ctr: 0,
        players: Vec::<Player>::new(),
        game_slots: Vec::<GameSlot>::new()
    };

    let iter_stream = stream::iter_ok::<_, Error>(iter::repeat( () ));
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
                        server_state.decode_packet(addr, packet.clone());

                        // Ack back
                        let ack_packet = Packet {
                            player_name: "from server".to_owned(),
                            number:      packet.number,
                            action:      RequestAction::Ack,
                        };
                        let response = (addr.clone(), ack_packet);
                        tx.unbounded_send(response).unwrap();
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

