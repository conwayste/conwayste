extern crate futures;
#[macro_use]
extern crate tokio_core;

use std::{env, io};
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

struct Server {
    socket:  UdpSocket,
    buf:     Vec<u8>,
    to_send: Option<(usize, SocketAddr)>,
}

impl Future for Server {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        let mut n = 0;
        loop {
            n += 1;
            println!("loop {} within poll", n);
            // First we check to see if there's a message we need to echo back.
            // If so then we try to send it back to the original source, waiting
            // until it's writable and we're able to do so.
            if let Some((size, peer)) = self.to_send {
                let amt = try_nb!(self.socket.send_to(&self.buf[..size], &peer));
                println!("Echoed {}/{} bytes to {}", amt, size, peer);
                self.to_send = None;
            }

            // If we're here then `to_send` is `None`, so we take a look for the
            // next message we're going to echo back.
            self.to_send = Some(try_nb!(self.socket.recv_from(&mut self.buf)));
            println!("got to end of loop {}", n);
        }
    }
}

fn main() {
    let addr = env::args().nth(1).unwrap_or("[::1]:12345".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();

    // Create the event loop that will drive this server, and also bind the socket we'll be
    // listening to.
    let mut l = Core::new().unwrap();
    let handle = l.handle();
    let socket = UdpSocket::bind(&addr, &handle).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    // Next we'll create a future to spawn (the one we defined above) and then we'll run the event
    // loop by running the future.
    l.run(Server {
        socket: socket,
        buf: vec![0; 1024],
        to_send: None,
    }).unwrap();
}
