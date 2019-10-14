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

#[macro_use]
pub(crate) mod helpe;
mod button;
mod chatbox;
mod checkbox;
mod label;
mod layer;
mod pane;
mod widget;
mod textfield;

use crate::Screen;

pub use button::Button;
pub use chatbox::Chatbox;
pub use checkbox::Checkbox;
pub use helpe::{
    within_widget,
    center,
    draw_text,
    intersection,
    point_offset
    };
pub use label::Label;
pub use layer::Layer;
pub use pane::Pane;
pub use textfield::{TextField, TextInputState};
pub use widget::Widget;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum UIAction {
    ScreenTransition(Screen),
    Toggle(bool),
    EnterText,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct WidgetID(pub usize);
