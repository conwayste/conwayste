#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;
extern crate chrono;

use std::env;
use std::io::{self, Read, Write};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use netwayste::net::{
    Packet, LineCodec, bind,
};
use netwayste::client::{
    ClientState, UserInput, parse_stdin
};
use tokio_core::reactor::{Core, Timeout};
use futures::{Future, Sink, Stream, stream, future::ok, sync::mpsc};
use log::LevelFilter;
use chrono::Local;

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

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:12345".to_owned());
    let addr = addr.parse::<SocketAddr>()
       .unwrap_or_else(|e| {
                    error!("failed to parse address {:?}: {:?}", addr, e);
                    exit(1);
                });
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

    trace!("Accepting commands to remote {:?} from local {:?}.", addr, local_addr);
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
    }).map_err(|_| ());

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::Incoming(packet_tuple)
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

    let network_stream = stream::iter_ok::<_, io::Error>(iter::repeat( () ));
    let network_stream = network_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(NETWORK_INTERVAL_IN_MS), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::NetworkEvent)
        })
    }).map_err(|_| ());

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
#[cfg(test)]
mod test {
}
