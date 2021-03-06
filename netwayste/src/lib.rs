/*  Copyright 2019 the Conwayste Developers.
 *
 *  This file is part of netwayste.
 *
 *  netwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  netwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with netwayste.  If not, see <http://www.gnu.org/licenses/>. */
#![recursion_limit = "512"] // The select!{...} macro hits the default 128 limit

extern crate base64;
extern crate bincode;
extern crate chrono;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate clap;
extern crate rand;
extern crate semver;
extern crate serde;

#[macro_use]
pub mod net;
pub mod client;
pub mod utils;

#[cfg(test)]
pub mod tests;
