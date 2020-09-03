/*  Copyright 2018-2020 the Conwayste Developers.
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

use ggez::graphics::{self, Rect, Scale};
use std::time::Duration;

// Universe settings
pub const UNIVERSE_WIDTH_IN_CELLS: usize = 256;
pub const UNIVERSE_HEIGHT_IN_CELLS: usize = 120;
pub const INTRO_UNIVERSE_WIDTH_IN_CELLS: usize = 256;
pub const INTRO_UNIVERSE_HEIGHT_IN_CELLS: usize = 256;

// game play
pub const CURRENT_PLAYER_ID: usize = 1; // TODO:  get the player ID from server rather than hardcoding
pub const FOG_RADIUS: usize = 4; // cells
pub const HISTORY_SIZE: usize = 16;

// Colors
pub mod colors {
    use crate::ui::common::color_with_alpha;
    use chromatica::css;
    use ggez::graphics::Color;

    lazy_static! {
        // To see what the colors look like: https://developer.mozilla.org/en-US/docs/Web/CSS/color_value#Color_keywords
        // TODO: probably can consoldate/remove many of these once the design is fleshed out more
        pub static ref INPUT_TEXT_COLOR: Color = Color::from(css::DARKRED);
        pub static ref CHATBOX_TEXT_COLOR: Color = Color::from(css::DARKRED);
        pub static ref CHATBOX_BORDER_COLOR: Color = Color::from(css::FIREBRICK);
        pub static ref CHATBOX_INACTIVE_BORDER_COLOR: Color = color_with_alpha(css::VIOLET, 0.5);
        pub static ref CHATBOX_BORDER_ON_HOVER_COLOR: Color = Color::from(css::TEAL);
        pub static ref MENU_TEXT_COLOR: Color = Color::from(css::WHITE);
        pub static ref MENU_TEXT_SELECTED_COLOR: Color = Color::from(css::LIME);
        pub static ref CHECKBOX_TEXT_COLOR: Color = Color::from(css::WHITE);
        pub static ref CHECKBOX_BORDER_ON_HOVER_COLOR: Color = Color::from(css::VIOLET);
        pub static ref CHECKBOX_TOGGLED_FILL_COLOR: Color = Color::from(css::AZURE);
        pub static ref CHAT_PANE_FILL_COLOR: Color = color_with_alpha(css::TURQUOISE, 0.33);
        pub static ref PANE_BORDER_COLOR: Color = Color::from(css::FIREBRICK);
        pub static ref CELL_STATE_DEAD_COLOR: Color = Color::new(0.875, 0.875, 0.875, 1.0);
        pub static ref CELL_STATE_BG_FILL_SOLID_COLOR: Color = Color::from(css::WHITE);
        pub static ref CELL_STATE_BG_FILL_HOLLOW_COLOR: Color = Color::from(css::BLACK);
        pub static ref CELL_STATE_ALIVE_PLAYER_0_COLOR: Color = Color::from(css::RED);
        pub static ref CELL_STATE_ALIVE_PLAYER_1_COLOR: Color = Color::from(css::BLUE);
        pub static ref CELL_STATE_WALL_COLOR: Color = Color::new(0.617, 0.55, 0.41, 1.0);
        pub static ref CELL_STATE_FOG_COLOR: Color = Color::new(0.780, 0.780, 0.780, 1.0);
        pub static ref GEN_COUNTER_COLOR: Color = Color::from(css::RED);
        pub static ref UNIVERSE_BG_COLOR: Color = Color::new( 0.25,  0.25,  0.25, 1.0);
        pub static ref LAYER_TRANSPARENCY_BG_COLOR: Color = color_with_alpha(css::HONEYDEW, 0.4);
        pub static ref OPTIONS_TEXT_FILL_COLOR: Color = Color::from(css::YELLOW);
        pub static ref OPTIONS_LABEL_TEXT_COLOR: Color = Color::from(css::WHITE);
        pub static ref INSERT_PATTERN_UNWRITABLE: Color = Color::from(css::RED);
    }
}

pub const DEFAULT_SCREEN_HEIGHT: f32 = 800.0; // pixels
pub const DEFAULT_SCREEN_WIDTH: f32 = 1200.0; // pixels
pub const DEFAULT_ZOOM_LEVEL: f32 = 5.0; // default cell size in pixels
                                         //pub const FPS: u32 = 25;
pub const GRID_DRAW_STYLE: DrawStyle = DrawStyle::Fill;
pub const INTRO_DURATION: f64 = 8.0; // seconds
pub const INTRO_PAUSE_DURATION: f64 = 3.0; // seconds
pub const MAX_CELL_SIZE: f32 = 40.0; // pixels
pub const MIN_CELL_SIZE: f32 = 5.0; // pixels
pub const PIXELS_SCROLLED_PER_FRAME: f32 = 50.0; // pixels

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
    pub static ref DEFAULT_CHATBOX_RECT: Rect =  Rect::new(30.0, 40.0, 300.0, 175.0);

}
// Border thickness of chatbox in pixels.
pub const CHATBOX_BORDER_PIXELS: f32 = 1.0;
pub const CHATBOX_LINE_SPACING: f32 = 2.0;
pub const CHATBOX_HISTORY: usize = 20;
pub const CHAT_TEXTFIELD_HEIGHT: f32 = 25.0;

// Layering's tree data structure capacities. Arbitrarily chosen.
pub const LAYERING_NODE_CAPACITY: usize = 100;
pub const LAYERING_SWAP_CAPACITY: usize = 10;

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
