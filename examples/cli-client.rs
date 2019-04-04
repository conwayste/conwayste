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
use netwayste::{net, client::ClientNetState};
use futures::sync::mpsc;
use tokio_core::reactor::Core;

//////////////////// Main /////////////////////
fn main() {
    let mut core = Core::new().unwrap();
    let remote = core.remote();

    let (_, b) = mpsc::channel::<net::RequestAction>(1);
    let (c, d) = mpsc::channel::<net::ResponseCode>(1);
    thread::spawn(move || {
        ClientNetState::start_network(c, b);
    });

    loop {}
}