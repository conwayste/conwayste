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
    let stream_map = stream.map(|(addr, packet)| {
        //XXX
        (addr, packet) //XXX echo???
    });

    // https://docs.rs/futures/0.1.14/futures/sink/trait.Sink.html#method.send_all :
    //   "This future will drive the stream to keep producing items until it is exhausted, sending
    //   each item to the sink. It will complete once both the stream is exhausted, the sink has
    //   received all items, the sink has been flushed, and the sink has been closed."
    let sink_future = sink.send_all(stream_map);

    drop(core.run(sink_future));
}

