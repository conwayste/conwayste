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

// FIFO to handle key inputs chronologically
// Each input may result in different actions based on the game stage
//

use std::collections::VecDeque;
use ggez::event::{Keycode, MouseButton};
//use ggez::event::{Button, Axis};

const NUM_OF_INPUTS: usize = 10;

pub enum InputDeviceType {
    PRIMARY,
//    GAMEPAD,
}

#[derive(Debug)]
pub enum InputAction {
    KeyPress(Keycode, bool),
    KeyRelease(Keycode),

    MouseClick(MouseButton, i32, i32),
    MouseDrag(MouseButton, i32, i32),
    MouseRelease(MouseButton),
    MouseMovement(i32, i32),

//    Gamepad((Button, Axis)),
}

pub struct InputManager {
    _device: InputDeviceType,
    events: VecDeque<InputAction>,
}

impl InputManager {
    pub fn new(device_type: InputDeviceType) -> InputManager {
        InputManager {
            _device: device_type,
            events: VecDeque::<InputAction>::new(),
        }
    }

    pub fn add(&mut self, input: InputAction) {
        // Curious to see if we actually can hit this condition
        if self.events.len() >= NUM_OF_INPUTS {
            println!("{:?}", self.events);
            assert!(false);
        }

        self.events.push_back(input);
    }

    pub fn _peek_next(&self) -> Option<&InputAction> {
        self.events.front()
    }
    
    pub fn remove(&mut self) -> Option<InputAction> {
        self.events.pop_front()
    }

    pub fn expunge(&mut self) {
        self.events.clear();
    }

    pub fn has_more(&mut self) -> bool {
        !self.events.is_empty()
    }

}
