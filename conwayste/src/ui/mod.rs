/*  Copyright 2019-2020 the Conwayste Developers.
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

extern crate chromatica;
extern crate env_logger;
extern crate ggez;

#[macro_use]
pub(crate) mod common;
#[macro_use]
pub(crate) mod context;
mod button;
mod chatbox;
mod checkbox;
mod focus;
mod gamearea;
mod label;
mod layer;
mod pane;
mod textfield;
mod treeview;
pub(crate) mod ui_errors;
mod widget;

pub use button::Button;
pub use chatbox::{Chatbox, ChatboxPublishHandle};
pub use checkbox::Checkbox;
pub use common::{center, color_with_alpha, draw_text, intersection, point_offset, within_widget};
pub use context::{EmitEvent, Event, EventType, UIContext};
pub use gamearea::GameArea;
pub use label::Label;
pub use layer::{InsertLocation, Layering};
pub use pane::Pane;
pub use textfield::TextField;
pub use ui_errors::{UIError, UIResult};
pub use widget::Widget;

type BoxedWidget = Box<dyn Widget>;
