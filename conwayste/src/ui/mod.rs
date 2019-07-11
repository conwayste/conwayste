/*  Copyright 2019 the Conwayste Developers.
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

extern crate ggez;
extern crate chromatica;

mod button;
mod chatbox;
mod checkbox;
mod pane;
mod helpe;
mod widget;
mod label;

pub use button::Button;
pub use chatbox::{Chatbox, TextState};
pub use checkbox::{Checkbox, ToggleState};
pub use helpe::{
    within_widget,
    center,
    draw_text,
    intersection,
    point_offset
    };
pub use label::Label;
pub use pane::Pane;
pub use widget::Widget;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Screen {
    Intro(f64),   // seconds
    Menu,
    ServerList,
    InRoom,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum UIAction {
    ScreenTransition(Screen),
    Toggle(ToggleState),
    EnterText(TextState),
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum WidgetID {
    MainMenuTestButton,
    MainMenuTestCheckbox,

    MainMenuPane1,
    MainMenuPane1ButtonYes,

    InGamePane1Chatbox
}
