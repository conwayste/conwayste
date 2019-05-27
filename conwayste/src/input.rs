/*  Copyright 2017-2019 the Conwayste Developers.
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

use std::time::{Instant};
use ggez::event::{Keycode, Mod, MouseButton};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ScrollEvent {
    ScrollUp, // Away from the user
    ScrollDown, // Towards the user
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MouseAction {
    Held,
    Drag,
    Click,
}

pub struct MouseInfo {
    pub mousebutton: MouseButton,
    pub action: Option<MouseAction>,
    pub scroll_event: Option<ScrollEvent>,
    pub down_timestamp: Option<Instant>,
    pub position: (i32, i32),
    pub debug_print: bool
}

impl MouseInfo {
    fn new() -> Self {
        MouseInfo {
            mousebutton: MouseButton::Unknown,
            action: None,
            scroll_event: None,
            down_timestamp: None,
            position: (0, 0),
            debug_print: false,
        }
    }

    #[allow(unused)]
    pub fn print_mouse_state(&mut self) {
        if self.debug_print {
            println!("Button: {:?}", self.mousebutton);
            println!("Action: {:?}", self.action);
            println!("Scroll: {:?}", self.scroll_event);
            println!("Down TS: {:?}", self.down_timestamp);
            println!("Position: {:?}", self.position);
        }
    }
}

pub struct KeyInfo {
    pub key: Option<Keycode>,
    pub repeating: bool,
    pub modifier: Option<Mod>,
    pub debug_print: bool,
}

impl KeyInfo {
    fn new() -> Self {
        KeyInfo {
            key: None,
            repeating: false,
            modifier: None,
            debug_print: false,
        }
    }

    #[allow(unused)]
    pub fn print_keyboard_state(&mut self) {
        if self.debug_print {
            println!("Key: {:?}", self.key);
            println!("Modifier: {:?}", self.modifier);
            println!("Repeating: {:?}", self.repeating);
        }
    }
}

/// InputManager maps input from devices to in-game events.
pub struct InputManager {
    pub mouse_info: MouseInfo,
    pub key_info: KeyInfo,
}

impl InputManager {
    pub fn new() -> InputManager {
        InputManager {
            mouse_info: MouseInfo::new(),
            key_info: KeyInfo::new(),
        }
    }

}
