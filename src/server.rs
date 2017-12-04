#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;

mod net;

use net::Core;
use net::{Action, PlayerPacket, LineCodec, Stream};
use net::futures::*;
use std::process::exit;

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let sock = net::bind(&handle, None, None)
                .unwrap_or_else(|e| {
                    error!("Error while trying to bind UDP socket: {:?}", e);
                    exit(1);
                });
    //XXX need to move this boilerplate to net as much as possible
    let (sink, stream) = sock.framed(LineCodec).split();
    let server = stream.for_each(|(addr, opt_packet)| {
        println!("got {:?} and {:?}!", addr, opt_packet);
        //XXX use handle.spawn on a function that puts outgoing (addr, packet) tuples in the sink
        Ok(())
    });

    drop(core.run(server));
}

