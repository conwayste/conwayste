#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;

mod net;

use net::{Action, PlayerPacket, LineCodec};
use std::net::SocketAddr;
use std::process::exit;
use std::io;
use futures::*;
use futures::future::ok;
use tokio_core::reactor::Core; // Handle, Timeout too?


fn get_responses(addr: SocketAddr) -> Box<Future<Item = Vec<Result<(SocketAddr, PlayerPacket), std::io::Error>>, Error = std::io::Error>> {
    let mut source_packet = PlayerPacket {
        player_name: "from server".to_owned(),
        number:      1,
        action:      Action::Click,
    };
    let mut responses = Vec::<_>::new();
    for _ in 0..3 {
        let packet = source_packet.clone();
        responses.push(Ok((addr.clone(), packet)));
        source_packet.number += 1;
    }
    Box::new(ok(responses))
}

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let sock = net::bind(&handle, None, None)
                .unwrap_or_else(|e| {
                    error!("Error while trying to bind UDP socket: {:?}", e);
                    exit(1);
                });
    let (sink, stream) = sock.framed(LineCodec).split();
    let server = stream.and_then(|(addr, opt_packet)| {
        println!("got {:?} and {:?}!", addr, opt_packet);
        get_responses(addr)
    })
    .and_then(|responses| {
        let responses_stream = stream::iter_result(responses).map_err(|_| io::Error::new(io::ErrorKind::Other, "someerr"));
        let sender = sink.send_all(responses_stream)
            .then(|_| Ok(()));
        handle.spawn(sender);
        Ok(())
    })
    .for_each(|_| Ok(()));

    drop(core.run(server));
}

