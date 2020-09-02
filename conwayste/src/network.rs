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

extern crate futures;
extern crate ggez;
extern crate netwayste;
extern crate tokio;

use futures as Fut;

use netwayste::client::ClientNetState;
use netwayste::net::NetwaysteEvent;

pub struct ConwaysteNetWorker {
    sender:   Fut::channel::mpsc::UnboundedSender<NetwaysteEvent>,
    receiver: Fut::channel::mpsc::Receiver<NetwaysteEvent>,
}

impl ConwaysteNetWorker {
    // TODO: This will likely be refactored after the networking architecture update soon coming
    #[allow(unused)]
    pub fn new() -> Self {
        let (netwayste_request_sender, netwayste_request_receiver) = Fut::channel::mpsc::unbounded::<NetwaysteEvent>();
        let (netwayste_response_sender, netwayste_response_receiver) = Fut::channel::mpsc::channel::<NetwaysteEvent>(5);

        tokio::spawn(async {
            match ClientNetState::start_network(netwayste_response_sender, netwayste_request_receiver).await {
                Ok(()) => {}
                Err(e) => error!("Error during ClientNetState: {}", e),
            }
        });

        ConwaysteNetWorker {
            sender:   netwayste_request_sender,
            receiver: netwayste_response_receiver,
        }
    }

    pub fn try_send(&mut self, nw_event: NetwaysteEvent) {
        match self.sender.unbounded_send(nw_event) {
            Ok(_) => {}
            Err(e) => error!("Error occurred during send to the netwayste receiver: {:?}", e),
        }
    }

    /// Update handler call from Conwayste's main event hander.
    /// Manages all received network packets and sets them up to be handled as needed.AsMut
    ///
    /// Must not block or delay in any way as this will hold up the main event update loop!
    pub fn try_receive(&mut self) -> Vec<NetwaysteEvent> {
        let mut new_events = vec![];
        loop {
            match self.receiver.try_next() {
                Ok(Some(response)) => {
                    new_events.push(response);
                }
                Ok(None) => {
                    // do nothing
                    break;
                }
                Err(e) => {
                    error!(
                        "Communications channel link with netwayste disconnected unexpectedly. {} Shutting down...",
                        e
                    );
                    break;
                }
            }
        }
        new_events
    }
}
