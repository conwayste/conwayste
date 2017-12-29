// FIFO to handle key inputs chronologically
// Each input may result in different actions based on the game stage
//

use std::collections::VecDeque;
use ggez::event::{Keycode, MouseButton, Button, Axis};

const NUM_OF_INPUTS: usize = 10;

pub type PrimaryInput = (Option<Keycode>, Option<MouseButton>);
pub type GamepadInput = (Option<Button>, Option<Axis>);

pub enum InputDeviceType {
    PRIMARY,
    GAMEPAD,
    CHINCHILLA
}

pub enum InputDevices {
    PrimaryInput,
    GamepadInput,
}

#[derive(Debug)]
pub enum InputAction {
    KeyPress(Keycode),
    KeyRelease(Keycode),

    MouseClick(MouseButton, i32, i32),
    MouseRelease(MouseButton),
    MouseMovement(MouseButton),

//    Gamepad((Button, Axis)),
}

pub struct InputManager {
    device: InputDeviceType,
    events: VecDeque<InputAction>,
}

impl InputManager {
    pub fn new(deviceType: InputDeviceType) -> InputManager {
        InputManager {
            device: deviceType,
            events: VecDeque::<InputAction>::new(),
        }
    }

    pub fn add(&mut self, input: InputAction) {
        self.events.push_back(input);
    }

    pub fn peek_next(&self) -> Option<&InputAction> {
        self.events.front()
    }
    
    pub fn remove(&mut self) -> Option<InputAction> {
        self.events.pop_front()
    }

    pub fn expunge(&mut self) {
        self.events.clear();
    }

    pub fn handle_inputs(&mut self) {
        self.events.iter().for_each(|x| {
            match *x {
                InputAction::KeyPress(k) => println!("Key Pressed: {:?}", k),
                InputAction::KeyRelease(k) => println!("Key Released: {:?}", k),
                InputAction::MouseClick(b, x, y) => println!("Mouse Button: {:?}", b),
                _ => {},
            }
        });
    }

    pub fn print_raw(&self) {
        if !self.events.is_empty() {
            println!("{:?}", self.events);
        }
    }
}
