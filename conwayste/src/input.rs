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

    MouseClick(MouseButton, i32, i32),
    MouseDrag(MouseButton, i32, i32),
    MouseRelease(MouseButton),
    MouseMovement(i32, i32),

//    Gamepad((Button, Axis)),
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
        if self.events.len() >= NUM_OF_QUEUED_INPUTS {
            // FIXME
            panic!("BUG: Input queue is full! Queue contents: {:?}", self.events);
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


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_inputmanager() {
        let im = InputManager::new(InputDeviceType::PRIMARY);

        assert_eq!(im._device, InputDeviceType::PRIMARY);
        assert_eq!(im.events.len(), 0);
        assert_eq!(im.has_more(), false);
    }

    #[test]
    fn test_dequeue_empty_inputmanager() {
        let mut im = InputManager::new(InputDeviceType::PRIMARY);

        assert_eq!(im.has_more(), false);
        assert_eq!(im.remove(), None);
    }

    #[test]
    fn test_enqueue() {
        let mut im = InputManager::new(InputDeviceType::PRIMARY);

        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        im.add(action);
        
        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        assert_eq!(im.peek(), Some(&action));
    }

    #[test]
    fn test_dequeue() {
        let mut im = InputManager::new(InputDeviceType::PRIMARY);
        assert_eq!(im.has_more(), false);

        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        im.add(action);
        assert_eq!(im.has_more(), true);
        
        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        assert_eq!(im.remove(), Some(action));
        assert_eq!(im.has_more(), false);
    }

    #[test]
    #[should_panic]
    fn test_fill_up_queue() {
        let mut im = InputManager::new(InputDeviceType::PRIMARY);

        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        for _ in 0..NUM_OF_QUEUED_INPUTS+1{
            im.add(action.clone());
        }
    }

    #[test]
    fn test_clear_queue() {
        let mut im = InputManager::new(InputDeviceType::PRIMARY);

        let action = InputAction::MouseClick(MouseButton::Left, 10, 10);
        for _ in 0..NUM_OF_QUEUED_INPUTS{
            im.add(action.clone());
        }

        im.expunge();
        assert_eq!(im.remove(), None);
        assert_eq!(im.has_more(), false);
    }
}
