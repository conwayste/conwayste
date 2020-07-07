/*  Copyright 2017-2018 the Conwayste Developers.
 *
 *  This file is part of libconway.
 *
 *  libconway is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  libconway is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with libconway.  If not, see <http://www.gnu.org/licenses/>. */

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate custom_error;

pub mod error;
pub mod grids;
pub mod rle;
pub mod universe;

pub use error::{ConwayError, ConwayResult};

pub use grids::Rotation;

#[cfg(test)]
pub mod tests;
