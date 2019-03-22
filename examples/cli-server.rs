#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;
extern crate base64;
extern crate rand;
extern crate semver;
extern crate chrono;
#[macro_use]
extern crate netwayste;

use netwayste::net::{
    ResponseCode, Packet, LineCodec, bind
};

use netwayste::server::{
    ServerState, TICK_INTERVAL_IN_MS, HEARTBEAT_INTERVAL_IN_MS, NETWORK_INTERVAL_IN_MS
};

use std::io::{self, Write};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::time::{Duration};
use futures::{Future, Sink, Stream, stream, future::ok, sync::mpsc};
use tokio_core::reactor::{Core, Timeout};
use chrono::Local;
use log::LevelFilter;

//////////////// Event Handling /////////////////
#[allow(unused)]
enum Event {
    TickEvent,
    Request((SocketAddr, Option<Packet>)),
    NetworkEvent,
    HeartBeat,
//    Notify((SocketAddr, Option<Packet>)),
}

pub fn main() {
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

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (tx, rx) = mpsc::unbounded();

    let udp = bind(&handle, None, None)
        .unwrap_or_else(|e| {
            error!("Error while trying to bind UDP socket: {:?}", e);
            exit(1);
        });

    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();

    let initial_server_state = ServerState::new();

    let iter_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
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
            Event::Request(packet_tuple)
        })
        .map_err(|_| ());

    let network_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
    let network_stream = network_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(NETWORK_INTERVAL_IN_MS), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::NetworkEvent)
        })
    }).map_err(|_| ());

    let heartbeat_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
    let heartbeat_stream = heartbeat_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(HEARTBEAT_INTERVAL_IN_MS), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::HeartBeat)
        })
    }).map_err(|_| ());

    let server_fut = tick_stream
        .select(packet_stream)
        .select(network_stream)
        .select(heartbeat_stream)
        .fold(initial_server_state, move |mut server_state: ServerState, event: Event | {
            match event {
                Event::Request(packet_tuple) => {
                     // With the above filter, `packet` should never be None
                    let (addr, opt_packet) = packet_tuple;

                    // Decode incoming and send a Response to the Requester
                    if let Some(packet) = opt_packet {
                        let decode_result = server_state.decode_packet(addr, packet.clone());
                        if decode_result.is_ok() {
                            let opt_response_packet = decode_result.unwrap();

                            if let Some(response_packet) = opt_response_packet {
                                let response = (addr.clone(), response_packet);
                                netwayste_send!(tx, response, ("[EVENT::REQUEST] Immediate response failed."));
                            }
                        } else {
                            let err = decode_result.unwrap_err();
                            error!("Decoding packet failed, from {:?}: {:?}", addr, err);
                        }
                    }
                }

                Event::TickEvent => {
                    server_state.expire_old_messages_in_all_rooms();
                    let client_update_packets_result = server_state.construct_client_updates();
                    if client_update_packets_result.is_ok() {
                        let opt_update_packets = client_update_packets_result.unwrap();

                        if let Some(update_packets) = opt_update_packets {
                            for update in update_packets {
                                netwayste_send!(tx, update, ("[EVENT::TICK] Could not send client update."));
                            }
                        }
                    }

                    /*
                    for x in server_state.network_map.values() {
                        trace!("\n\n\nNETWORK QUEUE CAPACITIES\n-----------------------\nTX: {}\nRX: {}\n\n\n", x.tx_packets.as_queue_type().capacity(), x.rx_packets.as_queue_type().capacity());
                    }
                    */

                    server_state.remove_timed_out_clients();
                    server_state.tick  = 1usize.wrapping_add(server_state.tick);
                }

                Event::NetworkEvent => {
                    // Process players in rooms
                    server_state.process_buffered_packets_in_rooms();

                    // Process players in lobby
                    server_state.process_buffered_packets_in_lobby();

                    server_state.resend_expired_tx_packets(&tx);
                }

                Event::HeartBeat => {
                    for player in server_state.players.values() {
                        let keep_alive = Packet::Response {
                            sequence: 0,
                            request_ack: None,
                            code: ResponseCode::KeepAlive
                        };
                        netwayste_send!(tx, (player.addr, keep_alive), ("[EVENT::HEARTBEAT] Could not send to Player: {:?}", player));
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
