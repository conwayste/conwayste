extern crate futures;
extern crate env_logger;
extern crate tokio_core;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

use std::io;
use std::net::SocketAddr;
use std::str;

use futures::{Future, Stream, Sink};
use tokio_core::net::{UdpSocket, UdpCodec};
use tokio_core::reactor::Core;
use bincode::{serialize, deserialize, Infinite};


#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Action {
    Click,
    Delete,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct PlayerPacket {
    player_name: String,
    number:      u64,
    action:      Action,
}


struct LineCodec;
impl UdpCodec for LineCodec {
    type In = (SocketAddr, PlayerPacket);
    type Out = (SocketAddr, PlayerPacket);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        let decoded: PlayerPacket = deserialize(buf).unwrap();
        Ok((*addr, decoded))
    }

    fn encode(&mut self, (addr, player_packet): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        let encoded: Vec<u8> = serialize(&player_packet, Infinite).unwrap();
        into.extend(encoded);
        addr
    }
}

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();  // port 0 means "pick a port for me" :)

    // Bind both our sockets and then figure out what ports we got.
    let a = UdpSocket::bind(&addr, &handle).unwrap();
    let b = UdpSocket::bind(&addr, &handle).unwrap();
    let b_addr = b.local_addr().unwrap();

    // We're parsing each socket with the `LineCodec` defined above, and then we
    // `split` each codec into the sink/stream halves.
    let (a_sink, a_stream) = a.framed(LineCodec).split();
    let (b_sink, b_stream) = b.framed(LineCodec).split();

    // Start off by sending a ping from a to b, afterwards we just print out
    // what they send us and continually send pings
    // let pings = stream::iter((0..5).map(Ok));
    let first_packet = PlayerPacket {
        player_name: String::from("me"),
        number:      1234,
        action:      Action::Click,
    };
    let a = a_sink.send((b_addr, first_packet)).and_then(|a_sink| {
        let mut i = 0;
        let new_a_stream = a_stream.take(4).map(move |(addr, packet)| {
            i += 1;
            println!("[a] i={} recv: {:?}", i, packet);
            let out_packet = PlayerPacket {
                player_name: String::from("you"),
                number:      5678,
                action:      Action::Delete,
            };
            (addr, out_packet)
        });
        a_sink.send_all(new_a_stream)
    });

    // The second client we have will receive the pings from `a` and then send back pongs.
    let new_b_stream = b_stream.map(|(addr, packet)| {
        println!("[b].recv: {:?}", packet);
        let another_out_packet = PlayerPacket {
            player_name: String::from("idk"),
            number:      8765,
            action:      Action::Click,
        };
        (addr, another_out_packet)
    });
    let b = b_sink.send_all(new_b_stream);

    // Spawn the sender of pongs and then wait for our pinger to finish.
    handle.spawn(b.then(|_| Ok(())));
    drop(core.run(a));
}
