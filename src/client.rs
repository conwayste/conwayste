#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate tokio_core;

mod net;

use net::{Action, PlayerPacket, LineCodec};
use tokio_core::reactor::{Core, Handle}; // Timeout too?

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    //XXX
}
