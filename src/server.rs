#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;

mod net;

use net::{Action, PlayerPacket, LineCodec, Event};
use std::net::SocketAddr;
use std::process::exit;
use std::io::{Error};
use std::iter;
use std::time::Duration;
use futures::*;
use futures::future::ok;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Handle, Timeout};

struct ServerState {
    ctr: u64
}

fn get_responses(addr: SocketAddr) -> Box<Future<Item = Vec<(SocketAddr, PlayerPacket)>, Error = std::io::Error>> {
    let mut source_packet = PlayerPacket {
        player_name: "from server".to_owned(),
        number:      1,
        action:      Action::Click,
    };
    let mut responses = Vec::<_>::new();
    for _ in 0..3 {
        let packet = source_packet.clone();
        responses.push((addr.clone(), packet));
        source_packet.number += 1;
    }
    Box::new(ok(responses))
}

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

    let initial_server_state = ServerState { ctr: 0 };

    let iter_stream = stream::iter_ok::<_, Error>(iter::repeat( () ));
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(10), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::TickEvent)
        })
    }).map_err(|_| ());

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::PacketEvent(packet_tuple)
        })
        .map_err(|_| ());

    let server_fut = tick_stream
        .select(packet_stream)
        .fold((tx.clone(), initial_server_state), move |(tx, mut server_state), event| {
            match event {
                Event::PacketEvent(packet_tuple) => {
                    let (addr, opt_packet) = packet_tuple;
                    println!("got {:?} and {:?}!", addr, opt_packet);

                    let packet = PlayerPacket {
                        player_name: "from server".to_owned(),
                        number:      1,
                        action:      Action::Click,
                    };
                    let response = (addr.clone(), packet);
                    tx.unbounded_send(response).unwrap();
                }
                Event::TickEvent => {
                    // Server tick
                    // Likely spawn off work to handle server tasks here
                    server_state.ctr += 1;
                }
            }

            // return the updated client for the next iteration
            ok((tx, server_state))
        })
        .map(|_| ())
        .map_err(|_| ());
/*
    let server = udp_stream.and_then(|(addr, opt_packet)| {
            println!("got {:?} and {:?}!", addr, opt_packet);
            get_responses(addr)
        })
        .and_then(|responses| {
            for outgoing_item in responses {
                tx.unbounded_send(outgoing_item).unwrap();
            }
            Ok(())
        })
        .for_each(|_| Ok(()))
        .map_err(|_| ());
*/
    let sink_fut = rx.fold(udp_sink, |udp_sink, outgoing_item| {
            let udp_sink = udp_sink.send(outgoing_item).map_err(|_| ());    // this method flushes (if too slow, use send_all)
            udp_sink
        }).map(|_| ()).map_err(|_| ());

    let combined_fut = server_fut.map(|_| ())
        .select(sink_fut)
        .map(|_| ());   // wait for either server_fut or sink_fut to complete

    drop(core.run(combined_fut));
}

