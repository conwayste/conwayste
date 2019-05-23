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
//use ggez::event::{Button, Axis};

const NUM_OF_QUEUED_INPUTS: usize = 10;

#[derive(Debug, PartialEq)]
pub enum InputDeviceType {
    PRIMARY,
//    GAMEPAD,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputAction {
    KeyPress(Keycode, bool),
    KeyRelease(Keycode),
}

/// InputManager maps input from devices to in-game events.
pub struct InputManager {
    _device: InputDeviceType,
    events: VecDeque<InputAction>,
}

impl InputManager {
    /// Instantiates the InputManager for the specified device
    /// and handles the associated events.
    pub fn new(device_type: InputDeviceType) -> InputManager {
        InputManager {
            _device: device_type,
            events: VecDeque::<InputAction>::new(),
        }
    }

    /// Adds an event to the queue.
    /// This will panic if we fill up the queue past `NUM_OF_QUEUED_INPUTS`.
    pub fn add(&mut self, input: InputAction) {
        // Curious to see if we actually can hit this condition
        if self.events.len() >= NUM_OF_QUEUED_INPUTS {
            println!("{:?}", self.events);
            assert!(false);
        }

        self.events.push_back(input);
    }

    /// Pokes at the head of the queue and returns an event if available.
    pub fn peek(&self) -> Option<&InputAction> {
        self.events.front()
    }

    /// Dequeues can input event.
    pub fn remove(&mut self) -> Option<InputAction> {
        self.events.pop_front()
    }

    /// Clears all events received during this frame.
    pub fn expunge(&mut self) {
        self.events.clear();
    }

    /// Checks to see if there are any more input events in this frame to process.
    pub fn has_more(&self) -> bool {
        !self.events.is_empty()
    }

}
