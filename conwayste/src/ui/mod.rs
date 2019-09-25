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
mod helpe;
mod label;
mod layer;
mod pane;
mod widget;
mod textfield;

use crate::Screen;

pub use button::Button;
pub use chatbox::Chatbox;
pub use checkbox::{Checkbox, ToggleState};
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
    Toggle(ToggleState),
    EnterText,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct WidgetID(pub usize);

pub const MAINMENU_LAYER1: WidgetID = WidgetID(0x100);
pub const MAINMENU_TESTBUTTON: WidgetID = WidgetID(0x101);
pub const MAINMENU_TESTBUTTONLABEL: WidgetID = WidgetID(0x102);
pub const MAINMENU_TESTCHECKBOX: WidgetID = WidgetID(0x103);

pub const MAINMENU_PANE1: WidgetID = WidgetID(0x104);
pub const MAINMENU_PANE1_BUTTONYES: WidgetID = WidgetID(0x105);
pub const MAINMENU_PANE1_BUTTONYESLABEL: WidgetID = WidgetID(0x106);
pub const MAINMENU_PANE1_BUTTONNO: WidgetID = WidgetID(0x107);
pub const MAINMENU_PANE1_BUTTONNOLABEL: WidgetID = WidgetID(0x108);

pub const INGAME_LAYER1: WidgetID = WidgetID(0x109);
pub const INGAME_PANE1: WidgetID = WidgetID(0x10A);
pub const INGAME_PANE1_CHATBOX: WidgetID = WidgetID(0x10B);
pub const INGAME_PANE1_CHATBOXTEXTFIELD: WidgetID = WidgetID(0x10C);
