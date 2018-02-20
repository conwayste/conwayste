#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;
extern crate futures;

mod net;

use std::env;
use std::io::{self, Read, Write, Error};
use std::iter;
use std::net::SocketAddr;
use std::process::exit;
use std::thread;
use std::time::Duration;
use net::{Action, PlayerPacket, LineCodec};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::{Core, Handle, Timeout};
use futures::{Future, Sink, Stream, stream};
use futures::future::ok;
use futures::sync::mpsc;

struct ClientState {
    ctr: u64
}

enum Event {
    TickEvent,
    PacketEvent((SocketAddr, Option<PlayerPacket>)),
}

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
    let (stdin_tx, stdin_rx) = mpsc::channel(0);
    thread::spawn(|| read_stdin(stdin_tx));
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
    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();
    let (udp_tx, udp_rx) = mpsc::unbounded();    // create a channel because we can't pass the sink around everywhere
    println!("About to start sending to remote {:?} from local {:?}...", addr, local_addr);

    // initialize state
    let initial_client_state = ClientState { ctr: 0 };

    let iter_stream = stream::iter_ok::<_, Error>(iter::repeat(())); // just a Stream that emits () forever
    // .and_then is like .map except that it processes returned Futures
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(1000), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::TickEvent)
        })
    }).map_err(|_| ());

    let packet_stream = udp_stream.map(|packet_tuple| {
        Event::PacketEvent(packet_tuple)
    }).map_err(|_| ());

    let main_loop_fut = tick_stream
        .select(packet_stream)
        .fold((udp_tx.clone(), initial_client_state), move |(udp_tx, mut client_state), event| {
            match event {
                Event::PacketEvent(packet_tuple) => {
                    println!("Got packet from server! {:?}", packet_tuple);
                }
                Event::TickEvent => {
                    // send a packet with ctr
                    let packet = PlayerPacket {
                        player_name: "Joe".to_owned(),
                        number:      client_state.ctr,
                        action:      Action::Click
                    };
                    // send packet
                    udp_tx.unbounded_send((addr.clone(), packet));
                    println!("Sent a packet! ctr is {}", client_state.ctr);

                    // just for fun, change our client state
                    client_state.ctr += 1;
                }
            }

            // finally, return the updated client state for the next iteration
            ok((udp_tx, client_state))
        })
        .map(|_| ())
        .map_err(|_| ());

    // listen on the channel created above and send it to the UDP sink
    let sink_fut = udp_rx.fold(udp_sink, |udp_sink, outgoing_item| {
        udp_sink.send(outgoing_item).map_err(|_| ())    // this method flushes (if too slow, use send_all)
    }).map(|_| ()).map_err(|_| ());

    let combined_fut = sink_fut.select(main_loop_fut).map_err(|_| ());

    core.run(combined_fut).unwrap();
}



// Our helper method which will read data from stdin and send it along the
// sender provided.
fn read_stdin(mut tx: mpsc::Sender<Vec<u8>>) {
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
