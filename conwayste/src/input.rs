/*  Copyright 2017-2018 the Conwayste Developers.
 *
 *  This file is part of conwayste.
 *
 *  conwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  conwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with conwayste.  If not, see
 *  <http://www.gnu.org/licenses/>. */

use std::collections::VecDeque;

use ggez::event::{Keycode, MouseButton};

const NUM_OF_QUEUED_INPUTS: usize = 10;

/// InputManager maps input from devices to in-game events.
pub struct InputManager {
}

impl InputManager {
    /// Instantiates the InputManager for the specified device
    /// and handles the associated events.
    pub fn new() -> InputManager {
        InputManager {
        }
    }

}
