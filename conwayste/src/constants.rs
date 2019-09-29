/*  Copyright 2018 the Conwayste Developers.
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

use std::time::Duration;
use ggez::graphics::{self, Color, Scale};
use chromatica::css;
use crate::ui::helpe::color_with_alpha;

// game play
pub const CURRENT_PLAYER_ID: usize = 1; // TODO:  get the player ID from server rather than hardcoding
pub const FOG_RADIUS: usize = 4; // cells
pub const HISTORY_SIZE: usize = 16;

// Colors
// Menu active color
pub const DEFAULT_ACTIVE_COLOR: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 0.0,
    a: 1.0,
}; // menu
// Menu inactive color
pub const DEFAULT_INACTIVE_COLOR: Color = Color {
    r: 0.75,
    g: 0.75,
    b: 0.75,
    a: 1.0,
}; // menu
lazy_static! {
    pub static ref INPUT_TEXT_COLOR: Color = Color::from(css::DARKRED);
    pub static ref CHATBOX_TEXT_COLOR: Color = Color::from(css::DARKRED);
    pub static ref CHATBOX_BORDER_COLOR: Color = Color::from(css::VIOLET);
    pub static ref CHATBOX_INACTIVE_BORDER_COLOR: Color = color_with_alpha(css::VIOLET, 0.5);
}

pub const DEFAULT_SCREEN_HEIGHT: f32     =  800.0; // pixels
pub const DEFAULT_SCREEN_WIDTH: f32      = 1200.0; // pixels
pub const DEFAULT_ZOOM_LEVEL: f32        =    5.0; // default cell size in pixels
//pub const FPS: u32 = 25;
pub const GRID_DRAW_STYLE: DrawStyle     = DrawStyle::Fill;
pub const INTRO_DURATION: f64            =  8.0;   // seconds
pub const INTRO_PAUSE_DURATION: f64      =  3.0;   // seconds
pub const MAX_CELL_SIZE: f32             = 40.0;   // pixels
pub const MIN_CELL_SIZE: f32             =  5.0;   // pixels
pub const PIXELS_SCROLLED_PER_FRAME: f32 = 50.0;   // pixels

// persistent configuration
pub const CONFIG_FILE_PATH: &str = "conwayste.toml";
pub const MIN_CONFIG_FLUSH_TIME: Duration = Duration::from_millis(5000);

// user interface
lazy_static! {
    // In pixels, used for any UI element containing text (except for chatbox)
    pub static ref DEFAULT_UI_FONT_SCALE: Scale = Scale::uniform(20.0);
    // In pixels, used for the message container of the chatbox. Currently different from other UI
    // elements for experimentation.
    pub static ref DEFAULT_CHATBOX_FONT_SCALE: Scale = Scale::uniform(15.0);

}
// Border thickness of chatbox in pixels.
pub const CHATBOX_BORDER_PIXELS: f32 = 4.0;
// FIXME This is a temporary hack! Relies on ggez/issue/583 to be fixed so we can calculate the
// last index a character will occupy for any sized text-field.
pub const CHATBOX_INPUT_VISIBLE_END_INDEX : usize = 22;

pub mod widget_ids {
    use crate::ui::WidgetID;
    pub const MAINMENU_LAYER1: WidgetID = WidgetID(0);
    pub const MAINMENU_TESTBUTTON: WidgetID = WidgetID(1);
    pub const MAINMENU_TESTBUTTONLABEL: WidgetID = WidgetID(2);
    pub const MAINMENU_TESTCHECKBOX: WidgetID = WidgetID(3);
    pub const MAINMENU_PANE1: WidgetID = WidgetID(4);
    pub const MAINMENU_PANE1_BUTTONYES: WidgetID = WidgetID(5);
    pub const MAINMENU_PANE1_BUTTONYESLABEL: WidgetID = WidgetID(6);
    pub const MAINMENU_PANE1_BUTTONNO: WidgetID = WidgetID(7);
    pub const MAINMENU_PANE1_BUTTONNOLABEL: WidgetID = WidgetID(8);
    pub const INGAME_LAYER1: WidgetID = WidgetID(9);
    pub const INGAME_PANE1: WidgetID = WidgetID(10);
    pub const INGAME_PANE1_CHATBOX: WidgetID = WidgetID(11);
    pub const INGAME_PANE1_CHATBOXTEXTFIELD: WidgetID = WidgetID(12);
}

//////////////////////////////////////////////////////////////////////

// This enum is needed because there is no PartialEq on the graphics::DrawMode enum in ggez.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum DrawStyle {
    Fill,
    Line,
}

impl DrawStyle {
    pub fn to_draw_mode(&self) -> graphics::DrawMode {
        match self {
            &DrawStyle::Fill => graphics::DrawMode::fill(),
            &DrawStyle::Line => graphics::DrawMode::stroke(1.0),
        }
    }
}
