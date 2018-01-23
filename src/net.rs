pub extern crate futures;
pub extern crate tokio_core;
extern crate bincode;

use std::io;
use std::net;
pub use std::net::SocketAddr;
use std::str;

pub use self::futures::{Future, Stream, Sink};
use self::tokio_core::net::{UdpSocket, UdpCodec};
pub use self::tokio_core::reactor::{Core, Handle, Timeout};
use self::bincode::{serialize, deserialize, Infinite};


//////////////// Error handling ////////////////
#[derive(Debug)]
pub enum NetError {
    AddrParseError(net::AddrParseError),
    IoError(io::Error),
}

impl From<net::AddrParseError> for NetError {
    fn from(e: net::AddrParseError) -> Self {
        NetError::AddrParseError(e)
    }
}

impl From<io::Error> for NetError {
    fn from(e: io::Error) -> Self {
        NetError::IoError(e)
    }
}



//////////////// Packet (de)serialization ////////////////
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Action {
    Click,
    Delete,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct PlayerPacket {
    pub player_name: String,
    pub number:      u64,
    pub action:      Action,
}


pub struct LineCodec;
impl UdpCodec for LineCodec {
    type In = (SocketAddr, Option<PlayerPacket>);   // if 2nd element is None, it means deserialization failure
    type Out = (SocketAddr, PlayerPacket);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        match deserialize(buf) {
            Ok(decoded) => Ok((*addr, Some(decoded))),
            Err(e) => {
                println!("WARNING: error during packet deserialization: {:?}", e);
                Ok((*addr, None))
            }
        }
    }

    fn encode(&mut self, (addr, player_packet): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        let encoded: Vec<u8> = serialize(&player_packet, Infinite).unwrap();
        into.extend(encoded);
        addr
    }
}

//////////////// Network interface ////////////////

pub fn bind(handle: &Handle, opt_host: Option<&str>, opt_port: Option<u16>) -> Result<UdpSocket, NetError> {

    let host = if let Some(host) = opt_host { host } else { HOST };
    let port = if let Some(port) = opt_port { port } else { PORT };
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let sock = UdpSocket::bind(&addr, &handle)?;
    Ok(sock)
}

//XXX other functions

pub const HOST: &str = "0.0.0.0";
pub const PORT: u16 = 12345;
