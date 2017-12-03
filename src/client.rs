#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;

mod net;

use net::Core;
use net::{Action, PlayerPacket, LineCodec};

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    //XXX
}
