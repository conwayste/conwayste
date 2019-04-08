/*
 * A networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2018-2019 The Conwayste Developers
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

use std::thread;
use std::sync::mpsc::channel as std_channel;
use std::sync::mpsc::TryRecvError;
use std::process;
use std::time::Duration;
use netwayste::{net, client::ClientNetState};
use futures::sync::mpsc;

//////////////////// Main /////////////////////
fn main() {
    let (ggez_client_request, nw_client_request) = mpsc::unbounded::<net::RequestAction>();
    let (nw_server_response, ggez_server_response) = std_channel::<net::ResponseCode>();
    thread::spawn(move || {
        ClientNetState::start_network(nw_server_response, nw_client_request);
    });

    ggez_client_request.unbounded_send(net::RequestAction::Connect{name: "blah3".to_owned(), client_version: "0.0.1".to_owned()})
        .unwrap();
    loop {
        ggez_client_request.unbounded_send(net::RequestAction::ListRooms)
            .unwrap();
        loop {
            match ggez_server_response.try_recv() {
                Ok(response_code) => {
                    println!("sweet! we got a netwayste ResponseCode! {:?}", response_code);
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(e) => {
                    println!("got error from ResponseCode stream from netwayste thread: {:?}", e);
                    process::exit(1);
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}
