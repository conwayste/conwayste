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

use ggez::{event::EventHandler, Context, GameResult, GameError};

use netwayste::net;
use netwayste::client::{ClientNetState, CLIENT_VERSION};

use std::thread;
use std::sync::mpsc as std_channel;
use std::sync::mpsc::TryRecvError;
use std::process;

pub struct NetworkManager {
    sender: futures_channel::UnboundedSender<net::RequestAction>,
    receiver: std_channel::Receiver<net::ResponseCode>,
}

impl NetworkManager {
    pub fn new() -> Self {
        let (netwayste_request_sender, netwayste_request_receiver) = futures_channel::unbounded::<net::RequestAction>();
        let (netwayste_response_sender, netwayste_response_receiver) = std_channel::channel::<net::ResponseCode>();
        thread::spawn(move || {
            ClientNetState::start_network(netwayste_response_sender, netwayste_request_receiver);
        });

        NetworkManager {
            sender: netwayste_request_sender,
            receiver: netwayste_response_receiver
        }
    }

    pub fn connect(&mut self, name: String) {
        self.sender
            .unbounded_send(net::RequestAction::Connect{name: name, client_version: CLIENT_VERSION.to_owned()})
            .unwrap();
    }
}

impl EventHandler for NetworkManager {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Get connection status
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Update connection status on screen if not ok
        Ok(())
    }
}