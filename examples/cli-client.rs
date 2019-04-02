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
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;
extern crate chrono;
extern crate regex;

use std::env;
use std::io::{self, Read, Write};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use netwayste::net::{
    Packet, LineCodec, bind, DEFAULT_PORT
};
use netwayste::client::{
    ClientState, UserInput, parse_stdin
};
use tokio_core::reactor::{Core, Timeout};
use futures::{Future, Sink, Stream, stream, future::ok, sync::mpsc};
use log::LevelFilter;
use chrono::Local;
use regex::Regex;

const TICK_INTERVAL_IN_MS:          u64    = 1000;
const NETWORK_INTERVAL_IN_MS:       u64    = 1000;


enum Event {
    TickEvent,
    UserInputEvent(UserInput),
    Incoming((SocketAddr, Option<Packet>)),
    NetworkEvent,
//    NotifyAck((SocketAddr, Option<Packet>)),
}

////////////////// Utilities //////////////////

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

    let has_port_re = Regex::new(r":\d{1,5}$").unwrap(); // match a colon followed by number up to 5 digits (16-bit port)
    let mut server_str = env::args().nth(1).unwrap_or("localhost".to_owned());
    // if no port, add the default port
    if !has_port_re.is_match(&server_str) {
        debug!("Appending default port to {:?}", server_str);
        server_str = format!("{}:{}", server_str, DEFAULT_PORT);
    }

    // synchronously resolve DNS because... why not?
    trace!("Resolving {:?}...", server_str);
    let addr_vec = tokio_dns::resolve_sock_addr(&server_str[..]).wait()      // wait() is synchronous!!!
                    .unwrap_or_else(|e| {
                            error!("failed to resolve: {:?}", e);
                            exit(1);
                        });
    if addr_vec.len() == 0 {
        error!("resolution found 0 addresses");
        exit(1);
    }
    // TODO: support IPv6
    let addr_vec_len = addr_vec.len();
    let v4_addr_vec: Vec<_> = addr_vec.into_iter().filter(|addr| addr.is_ipv4()).collect(); // filter out IPv6
    if v4_addr_vec.len() < addr_vec_len {
        warn!("Filtered out {} IPv6 addresses -- IPv6 is not implemented.", addr_vec_len - v4_addr_vec.len() );
    }
    if v4_addr_vec.len() > 1 {
        // This is probably not the best option -- could pick based on ping time, random choice,
        // and could also try other ones on connection failure.
        warn!("Multiple ({:?}) addresses returned; arbitrarily picking the first one.", v4_addr_vec.len());
    }

    let addr = v4_addr_vec[0];

    trace!("Connecting to {:?}", addr);

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Have separate thread read from stdin
    let (stdin_tx, stdin_rx) = mpsc::unbounded::<Vec<u8>>();
    let stdin_rx = stdin_rx.map_err(|_| panic!()); // errors not possible on rx

    // Unwrap ok because bind will abort if unsuccessful
    let udp = bind(&handle, Some("0.0.0.0"), Some(0)).unwrap();
    let local_addr = udp.local_addr().unwrap();

    // Channels
    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();
    let (udp_tx, udp_rx) = mpsc::unbounded();    // create a channel because we can't pass the sink around everywhere
    let (exit_tx, exit_rx) = mpsc::unbounded();  // send () to exit_tx channel to quit the client

    trace!("Locally bound to {:?}.", local_addr);
    trace!("Will connect to remote {:?}.", addr);
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
    }).map_err(|e| {
        error!("Got error from tick stream: {:?}", e);
        exit(1);
    });

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::Incoming(packet_tuple)
        })
        .map_err(|e| {
            error!("Got error from packet_stream {:?}", e);
            exit(1);
        });

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
    }).map_err(|e| {
        error!("Got error from network_stream {:?}", e);
        exit(1);
    });

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
        udp_sink.send(outgoing_item).map_err(|e| {
                error!("Got error while attempting to send UDP packet: {:?}", e);
                exit(1);
            })
    }).map(|_| ()).map_err(|_| ());

    let exit_fut = exit_rx
                    .into_future()
                    .map(|_| ())
                    .map_err(|e| {
                                error!("Got error from exit_fut: {:?}", e);
                                exit(1);
                            });

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
#[cfg(test)]
mod test {
}
