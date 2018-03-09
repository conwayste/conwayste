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
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use net::{Action, PlayerPacket, LineCodec, Event};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::{Core, Handle, Timeout};
use futures::{Future, Sink, Stream, stream};
use futures::future::ok;
use futures::sync::mpsc;

struct ClientState {
    ctr: u64
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
    let (stdin_tx, stdin_rx) = mpsc::unbounded::<Vec<u8>>();
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

    // Channels
    let (udp_sink, udp_stream) = udp.framed(LineCodec).split();
    let (udp_tx, udp_rx) = mpsc::unbounded();    // create a channel because we can't pass the sink around everywhere

    println!("About to start sending to remote {:?} from local {:?}...", addr, local_addr);

    // initialize state
    let initial_client_state = ClientState { ctr: 0 };

    let iter_stream = stream::iter_ok::<_, Error>(iter::repeat( () )); // just a Stream that emits () forever
    // .and_then is like .map except that it processes returned Futures
    let tick_stream = iter_stream.and_then(|_| {
        let timeout = Timeout::new(Duration::from_millis(1000), &handle).unwrap();
        timeout.and_then(move |_| {
            ok(Event::TickEvent)
        })
    }).map_err(|_| ());

    let packet_stream = udp_stream
        .filter(|&(_, ref opt_packet)| {
            *opt_packet != None
        })
        .map(|packet_tuple| {
            Event::Response(packet_tuple)
        })
        .map_err(|_| ());

    let stdin_stream = stdin_rx
        .filter(|buf| {
            let input = String::from_utf8(buf.to_vec().clone()).unwrap();
            let input = input.trim();
            !input.is_empty() && input != ""
        })
        .map(|buf| {
            let string = String::from_utf8(buf).unwrap();
            let string = String::from_str(string.trim()).unwrap();
            Event::StdinEvent(string)
        }).map_err(|_| ());

    let main_loop_fut = tick_stream
        .select(packet_stream)
        .select(stdin_stream)
        .fold((udp_tx.clone(), initial_client_state), move |(udp_tx, mut client_state), event| {
            match event {
                Event::Response(packet_tuple) => {
                    println!("Got packet from server! {:?}", packet_tuple);
                }
                Event::Request(_) => {panic!("Why did we get a Request from Server?");}
                Event::TickEvent => {
                    /*
                    // send a packet with ctr
                    let act = if client_state.ctr == 0 {
                         Action::Connect
                    }
                    else {
                        Action::None
                    };
                    let packet = PlayerPacket {
                        player_name: "Joe".to_owned(),
                        number:      client_state.ctr,
                        action:      act
                    };
                    // send packet
                    let _ = udp_tx.unbounded_send((addr.clone(), packet));
                    println!("Sent a packet! ctr is {}", client_state.ctr);

                    // just for fun, change our client state
                    client_state.ctr += 1;
                    */
                }
                Event::StdinEvent(string) => {
                    let action = parse_stdin(&string);
                    match action {
                        Action::None => {
                            println!("Command not recognized: {}", string);
                        },
                        all_others => {
                            let packet = PlayerPacket {
                                player_name: "Joe".to_owned(),
                                number:      client_state.ctr,
                                action:      all_others
                            };
                            println!("User input: {:?}", string);
                            let _ = udp_tx.unbounded_send((addr.clone(), packet));
                            client_state.ctr += 1;
                        }
                    }
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

// At this point we should only have command or message to work with
fn parse_stdin(input: &String) -> Action {
   if input.get(0..1) == Some("/") {
        let mut input = input.clone();
        input.remove(0);

        let input = input.to_lowercase();
        let mut iter = input.split_whitespace();
        let mut action = Action::None;

        if let Some(command) = iter.next() {
            match command {
                "help" => {
                    action = Action::Help;
                }
                "connect" => {
                    action = Action::Connect;
                }
                _ => {}
            }
        }
        action
   }
   else {
        Action::Message
   }
}
