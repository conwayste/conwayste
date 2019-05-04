/*
 * Herein lies a networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2019 The Conwayste Developers
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate netwayste;
extern crate futures;
extern crate tokio_core;
extern crate ggez;

use futures::sync::mpsc as futures_channel;

use netwayste::net::NetwaysteEvent;
use netwayste::client::{ClientNetState, CLIENT_VERSION};

use std::process;
use std::thread;
use std::sync::mpsc as std_channel;
use std_channel::TryRecvError;


pub struct ConwaysteNetWorker {
    sender: futures_channel::UnboundedSender<NetwaysteEvent>,
    receiver: std_channel::Receiver<NetwaysteEvent>,
}

impl ConwaysteNetWorker {
    pub fn new() -> Self {
        let (netwayste_request_sender, netwayste_request_receiver) = futures_channel::unbounded::<NetwaysteEvent>();
        let (netwayste_response_sender, netwayste_response_receiver) = std_channel::channel::<NetwaysteEvent>();
        thread::spawn(move || {
            ClientNetState::start_network(netwayste_response_sender, netwayste_request_receiver);
        });

        ConwaysteNetWorker {
            sender: netwayste_request_sender,
            receiver: netwayste_response_receiver
        }
    }

    pub fn connect(&mut self, name: String) {
        self.sender
            .unbounded_send(NetwaysteEvent::Connect(name, CLIENT_VERSION.to_owned()))
            .unwrap();
    }

    pub fn try_send(&mut self, nw_event: NetwaysteEvent) {
        match self.sender.unbounded_send(nw_event) {
            Ok(_) => { },
            Err(e) => error!("Error occurred during send to the netwayste receiver: {:?}", e)
        }
    }

    /// Update handler call from Conwayste's main event hander.
    /// Manages all received network packets and sets them up to be handled as needed.AsMut
    ///
    /// Must not block or delay in any way as this will hold up the main event update loop!
    pub fn try_receive(&mut self) -> Vec<NetwaysteEvent> {
        let mut new_events = vec![];
        loop {
            match self.receiver.try_recv() {
                Ok(response) => {
                    new_events.push(response);
                },
                Err(TryRecvError::Empty) => {
                    // Nothing to do in the empty case
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    println!("Communications channel link with netwayste disconnected unexpectedly. Shutting down...");
                    process::exit(1);
                }
            }
        }
        new_events
    }

}