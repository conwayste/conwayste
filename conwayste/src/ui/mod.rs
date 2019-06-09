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
mod checkbox;
mod chatbox;
mod helpe;
mod widget;
mod label;

pub use button::Button;
pub use checkbox::Checkbox;
pub use chatbox::Chatbox;
pub use helpe::{
    within_widget,
    center,
    draw_text,
    intersection,
    point_offset
    };
pub use label::Label;
pub use widget::Widget;
