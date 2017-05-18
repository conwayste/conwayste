
/*  Copyright 2017 the ConWaysteTheEnemy Developers.
 *
 *  This file is part of ConWaysteTheEnemy.
 *
 *  ConWaysteTheEnemy is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  ConWaysteTheEnemy is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with ConWaysteTheEnemy.  If not, see
 *  <http://www.gnu.org/licenses/>. */

// TODOs
// Contextual logging
// Modularization
// Menu System
//

extern crate conway;
extern crate ggez;
#[macro_use]
extern crate version;

use ggez::conf;
use ggez::event::*;
use ggez::game::{Game, GameState};
use ggez::{GameResult, Context};
use ggez::graphics;
use ggez::graphics::{Rect, Point, Color};
use ggez::timer;
use std::time::Duration;
use std::fs::File;
use conway::{Universe, CellState, Region};
use std::collections::BTreeMap;


const FPS: u32 = 25;
const INTRO_DURATION: f64 = 2.0;
const DEFAULT_SCREEN_WIDTH: u32 = 1200;
const DEFAULT_SCREEN_HEIGHT: u32 = 800;
const PIXELS_SCROLLED_PER_FRAME: i32 = 50;
const ZOOM_LEVEL_MIN: u32 = 4;
const ZOOM_LEVEL_MAX: u32 = 20;
const HISTORY_SIZE: usize = 16;
const NUM_PLAYERS: usize = 2;

#[derive(PartialEq)]
enum Stage {
    Intro(f64),   // seconds
    #[allow(dead_code)] // TODO: Consider this for pause as well?
    Menu,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
}

#[derive(PartialEq)]
enum ZoomDirection {
    ZoomOut,
    ZoomIn
}

// All game state
struct MainState {
    small_font:          graphics::Font,
    intro_text:          graphics::Text,
    stage:               Stage,             // Where are we in the game (Intro/Menu Main/Running..)
    uni:                 Universe,          // Things alive and moving here
    first_gen_was_drawn: bool,              // The purpose of this is to inhibit gen calc until the first draw
    grid_view:           GridView,
    color_settings:      ColorSettings,
    running:             bool,

    // Input state
    single_step:         bool,
    arrow_input:         (i32, i32),
    drag_draw:           Option<CellState>,
    win_resize:          u8,
}

// Support non-alive/dead/bg colors
struct ColorSettings {
    cell_colors: BTreeMap<CellState, Color>,
    background: Color,
}

impl ColorSettings {
    fn get_color(&self, cell_or_none: Option<CellState>) -> Color {
        match cell_or_none {
            Some(cell) => self.cell_colors[&cell],
            None       => self.background
        }
    }
}


fn init_patterns(s: &mut MainState) -> Result<(), ()> {
    /*
    // R pentomino
    s.uni.toggle(16, 15, 0)?;
    s.uni.toggle(17, 15, 0)?;
    s.uni.toggle(15, 16, 0)?;
    s.uni.toggle(16, 16, 0)?;
    s.uni.toggle(16, 17, 0)?;
    */

    /*
    // Acorn
    s.uni.toggle(23, 19, 0)?;
    s.uni.toggle(24, 19, 0)?;
    s.uni.toggle(24, 17, 0)?;
    s.uni.toggle(26, 18, 0)?;
    s.uni.toggle(27, 19, 0)?;
    s.uni.toggle(28, 19, 0)?;
    s.uni.toggle(29, 19, 0)?;
    */


    // Simkin glider gun
    s.uni.toggle(100, 70, 0)?;
    s.uni.toggle(100, 71, 0)?;
    s.uni.toggle(101, 70, 0)?;
    s.uni.toggle(101, 71, 0)?;

    s.uni.toggle(104, 73, 0)?;
    s.uni.toggle(104, 74, 0)?;
    s.uni.toggle(105, 73, 0)?;
    s.uni.toggle(105, 74, 0)?;

    s.uni.toggle(107, 70, 0)?;
    s.uni.toggle(107, 71, 0)?;
    s.uni.toggle(108, 70, 0)?;
    s.uni.toggle(108, 71, 0)?;

    /* eater
    s.uni.toggle(120, 87, 0)?;
    s.uni.toggle(120, 88, 0)?;
    s.uni.toggle(121, 87, 0)?;
    s.uni.toggle(121, 89, 0)?;
    s.uni.toggle(122, 89, 0)?;
    s.uni.toggle(123, 89, 0)?;
    s.uni.toggle(123, 90, 0)?;
    */

    s.uni.toggle(121, 80, 0)?;
    s.uni.toggle(121, 81, 0)?;
    s.uni.toggle(121, 82, 0)?;
    s.uni.toggle(122, 79, 0)?;
    s.uni.toggle(122, 82, 0)?;
    s.uni.toggle(123, 79, 0)?;
    s.uni.toggle(123, 82, 0)?;
    s.uni.toggle(125, 79, 0)?;
    s.uni.toggle(126, 79, 0)?;
    s.uni.toggle(126, 83, 0)?;
    s.uni.toggle(127, 80, 0)?;
    s.uni.toggle(127, 82, 0)?;
    s.uni.toggle(128, 81, 0)?;

    s.uni.toggle(131, 81, 0)?;
    s.uni.toggle(131, 82, 0)?;
    s.uni.toggle(132, 81, 0)?;
    s.uni.toggle(132, 82, 0)?;

    //Wall!
    s.uni.set(25, 18, CellState::Wall);
    s.uni.set(25, 17, CellState::Wall);
    s.uni.set(25, 16, CellState::Wall);
    s.uni.set(25, 15, CellState::Wall);
    s.uni.set(25, 14, CellState::Wall);
    s.uni.set(25, 13, CellState::Wall);
    s.uni.set(25, 12, CellState::Wall);
    s.uni.set(25, 11, CellState::Wall);
    s.uni.set(25, 10, CellState::Wall);
    s.uni.set(24, 10, CellState::Wall);
    s.uni.set(23, 10, CellState::Wall);
    s.uni.set(22, 10, CellState::Wall);
    s.uni.set(21, 10, CellState::Wall);
    s.uni.set(20, 10, CellState::Wall);
    s.uni.set(19, 10, CellState::Wall);
    s.uni.set(18, 10, CellState::Wall);
    s.uni.set(17, 10, CellState::Wall);
    s.uni.set(16, 10, CellState::Wall);
    s.uni.set(15, 10, CellState::Wall);
    s.uni.set(14, 10, CellState::Wall);
    s.uni.set(13, 10, CellState::Wall);
    s.uni.set(12, 10, CellState::Wall);
    s.uni.set(11, 10, CellState::Wall);
    s.uni.set(10, 10, CellState::Wall);
    s.uni.set(10, 11, CellState::Wall);
    s.uni.set(10, 12, CellState::Wall);
    s.uni.set(10, 13, CellState::Wall);
    s.uni.set(10, 14, CellState::Wall);
    s.uni.set(10, 15, CellState::Wall);
    s.uni.set(10, 16, CellState::Wall);
    s.uni.set(10, 17, CellState::Wall);
    s.uni.set(10, 18, CellState::Wall);
    s.uni.set(10, 19, CellState::Wall);
    s.uni.set(10, 20, CellState::Wall);
    s.uni.set(10, 21, CellState::Wall);
    s.uni.set(10, 22, CellState::Wall);
    s.uni.set(11, 22, CellState::Wall);
    s.uni.set(12, 22, CellState::Wall);
    s.uni.set(13, 22, CellState::Wall);
    s.uni.set(14, 22, CellState::Wall);
    s.uni.set(15, 22, CellState::Wall);
    s.uni.set(16, 22, CellState::Wall);
    s.uni.set(17, 22, CellState::Wall);
    s.uni.set(18, 22, CellState::Wall);
    s.uni.set(19, 22, CellState::Wall);
    s.uni.set(20, 22, CellState::Wall);
    s.uni.set(21, 22, CellState::Wall);
    s.uni.set(22, 22, CellState::Wall);
    s.uni.set(23, 22, CellState::Wall);
    s.uni.set(24, 22, CellState::Wall);
    s.uni.set(25, 22, CellState::Wall);
    Ok(())
}


// Then we implement the `ggez::game::GameState` trait on it, which
// requires callbacks for creating the game state, updating it each
// frame, and drawing it.
//
// The `GameState` trait also contains callbacks for event handling
// that you can override if you wish, but the defaults are fine.
impl GameState for MainState {

    fn load(ctx: &mut Context, _conf: &conf::Conf) -> GameResult<MainState> {
        let intro_font = graphics::Font::new(ctx, "DejaVuSerif.ttf", 48).unwrap();
        let intro_text = graphics::Text::new(ctx, "WAYSTE EM!", &intro_font).unwrap();

        let game_width  = 64*4; // num of cells * pixels per cell
        let game_height = 30*4;

        let grid_view = GridView {
            rect:        Rect::new(0, 0, DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT),
            cell_size:   10,
            columns:     game_width,
            rows:        game_height,
            grid_origin: Point::new(0, 0),
        };

        let mut color_settings = ColorSettings {
            cell_colors: BTreeMap::new(),
            background:  Color::RGB( 64,  64,  64),
        };
        color_settings.cell_colors.insert(CellState::Dead,           Color::RGB(224, 224, 224));
        color_settings.cell_colors.insert(CellState::Alive(None),    Color::RGB(  0,   0,   0));
        color_settings.cell_colors.insert(CellState::Alive(Some(0)), Color::RGB(255,   0,   0));  // 0 is red
        color_settings.cell_colors.insert(CellState::Alive(Some(1)), Color::RGB(  0,   0, 255));  // 1 is blue
        color_settings.cell_colors.insert(CellState::Wall,           Color::RGB(158, 141, 105));
        color_settings.cell_colors.insert(CellState::Fog,            Color::RGB(128, 128, 128));

        // we're going to have to tear this all out when this becomes a real game
        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];

        let small_font = graphics::Font::new(ctx, "DejaVuSerif.ttf", 20).unwrap();
        let mut s = MainState {
            small_font:          small_font,
            intro_text:          intro_text,
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 Universe::new(game_width, game_height, true, HISTORY_SIZE, NUM_PLAYERS, writable_regions).unwrap(),
            first_gen_was_drawn: false,
            grid_view:           grid_view,
            color_settings:      color_settings,
            running:             false,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            win_resize:          0,
        };

        init_patterns(&mut s).unwrap();


        Ok(s)
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) -> GameResult<()> {
        let duration = timer::duration_to_f64(dt); // seconds

        match self.stage {
            Stage::Intro(mut remaining) => {
                remaining -= duration;
                if remaining > 0.0 {
                    self.stage = Stage::Intro(remaining);
                } else {
                    self.stage = Stage::Run;
                }
            }
            Stage::Menu => {
                // TODO
            }
            Stage::Run => {
                if self.single_step {
                    self.running = false;
                }

                if self.first_gen_was_drawn && (self.running || self.single_step) {
                    self.uni.next();     // next generation
                    self.single_step = false;
                }

                let renderer = &mut _ctx.renderer;
                let mut x = DEFAULT_SCREEN_WIDTH;
                let mut y = DEFAULT_SCREEN_HEIGHT;

                // Resolution resizing
                // This is a temporary placeholder for this functionality until we implement
                // the video settings in Menu. 
                if self.win_resize != 0 {
                    match self.win_resize % 4 {
                        1 => { x = 640; y = 480; }
                        2 => { x = 800; y = 480; }
                        3 => { x = DEFAULT_SCREEN_WIDTH; y = DEFAULT_SCREEN_HEIGHT; }
                        _ => {}
                    }

                    let _ = renderer.set_logical_size(x,y);
                    {
                        let window = renderer.window_mut().unwrap();
                        let _ = window.set_size(x,y);
                    }
                }
                self.win_resize = 0;

                // Update panning
                if self.arrow_input != (0, 0) {
                    let cell_size = self.grid_view.cell_size;
                    let (columns, rows) = (self.grid_view.columns as u32, self.grid_view.rows as u32);
                    let screen_width  = cell_size*columns;
                    let screen_height = cell_size*rows;

                    let (dx, dy) = self.arrow_input;
                    let dx_in_pixels = -dx * PIXELS_SCROLLED_PER_FRAME;
                    let dy_in_pixels = -dy * PIXELS_SCROLLED_PER_FRAME;

                    let cur_origin_x = self.grid_view.grid_origin.x();
                    let cur_origin_y = self.grid_view.grid_origin.y();

                    let new_origin_x = cur_origin_x + dx_in_pixels;
                    let new_origin_y = cur_origin_y + dy_in_pixels;

                    let mut border_in_px = 100;

                    //if cell_size <= ZOOM_LEVEL_MIN {
                    //    border_in_px = 25;
                    //}

                    println!("Cell Size: {:?}", (cell_size, border_in_px));

                    // lol this works for now. One thing we'll need to check,
                    // either during zooming in or panning,
                    // is to check if our grid origin is out of bounds, and correct.
                    // Todo "11" & "7" are currently magical. TODO Align to resolution

                    let mut right_boundary_in_px = -1*(screen_width as i32 - border_in_px*11);
                    let mut bottom_boundary_in_px = -1*(screen_height as i32 - border_in_px*7);

                    if cell_size <= (ZOOM_LEVEL_MIN +1) {
                        right_boundary_in_px = -1*(200)*(cell_size - ZOOM_LEVEL_MIN+1) as i32;
                        bottom_boundary_in_px = -1*(100)*(cell_size - ZOOM_LEVEL_MIN+1) as i32;
                    }

                    if  new_origin_x > right_boundary_in_px
                     && new_origin_y > bottom_boundary_in_px
                     && new_origin_x < border_in_px
                     && new_origin_y < border_in_px {
                        self.grid_view.grid_origin = self.grid_view.grid_origin.offset(dx_in_pixels, dy_in_pixels);
                    }

                    if true {
                        println!("New Origin: {:?}", (new_origin_x, new_origin_y));
                        println!("Boundary: {:?}", (right_boundary_in_px, bottom_boundary_in_px));
                    }

                    // Snap edges in case we are out of bounds
                    if new_origin_x >= border_in_px {
                        self.grid_view.grid_origin = Point::new(border_in_px-1, cur_origin_y);
                    }

                    if new_origin_x <= right_boundary_in_px {
                        self.grid_view.grid_origin = Point::new(right_boundary_in_px+1, cur_origin_y);
                    }

                    //if cell_size != ZOOM_LEVEL_MIN {
                        if new_origin_y <= bottom_boundary_in_px {
                            self.grid_view.grid_origin = Point::new(cur_origin_x, bottom_boundary_in_px+1);
                        }

                        if new_origin_y >= border_in_px {
                            self.grid_view.grid_origin = Point::new(cur_origin_x, border_in_px-1);
                        }
                  //  }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        ctx.renderer.clear();
        match self.stage {
            Stage::Intro(_) => {
                try!(graphics::draw(ctx, &mut self.intro_text, None, None));
            }
            Stage::Menu => {
                // TODO 
            }
            Stage::Run => {
                ////////// draw universe
                // grid background
                graphics::set_color(ctx, self.color_settings.get_color(None));
                graphics::rectangle(ctx,  graphics::DrawMode::Fill, self.grid_view.rect).unwrap();

                // grid foreground (dead cells)
                //TODO: put in its own function (of GridView); also make this less ugly
                let origin = self.grid_view.grid_origin;
                let full_width  = self.grid_view.grid_width();
                let full_height = self.grid_view.grid_height();
                let full_rect = Rect::new(origin.x(), origin.y(), full_width, full_height);

                if let Some(clipped_rect) = full_rect.intersection(self.grid_view.rect) {
                    graphics::set_color(ctx, self.color_settings.get_color(Some(CellState::Dead)));
                    graphics::rectangle(ctx,  graphics::DrawMode::Fill, clipped_rect).unwrap();
                }

                // grid non-dead cells
                let visibility = Some(1); //XXX
                self.uni.each_non_dead_full(visibility, &mut |col, row, state| {
                    let color = self.color_settings.get_color(Some(state));
                    graphics::set_color(ctx, color);

                    if let Some(rect) = self.grid_view.window_coords_from_game(col, row) {
                        graphics::rectangle(ctx,  graphics::DrawMode::Fill, rect).unwrap();
                    }
                });


                ////////// draw generation counter
                let gen_counter_str = self.uni.latest_gen().to_string();
                let mut gen_counter_text = graphics::Text::new(ctx,
                                                               &gen_counter_str,
                                                               &self.small_font).unwrap();
                let dst = Rect::new(0, 0, gen_counter_text.width(), gen_counter_text.height());
                graphics::draw(ctx, &mut gen_counter_text, None, Some(dst))?;


                ////////////////////// END
                graphics::set_color(ctx, Color::RGB(0,0,0)); // do this at end; not sure why...?
                self.first_gen_was_drawn = true;
            }
        }
        ctx.renderer.present();
        timer::sleep_until_next_frame(ctx, FPS);
        Ok(())
    }

    fn mouse_button_down_event(&mut self, button: Mouse, x: i32, y: i32) {
        if button == Mouse::Left {
            if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                let result = self.uni.toggle(col, row, 1);   // TODO: don't hardcode the player number
                self.drag_draw = match result {
                    Ok(state) => Some(state),
                    Err(_)    => None,
                };
            }
        }
    }

    fn mouse_motion_event(&mut self, state: MouseState, x: i32, y: i32, _xrel: i32, _yrel: i32) {
        if state.left() && self.drag_draw != None {
            if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                let cell_state = self.drag_draw.unwrap();
                self.uni.set(col, row, cell_state);
            }
        }
    }

    fn mouse_button_up_event(&mut self, _button: Mouse, _x: i32, _y: i32) {
        // TODO Later, we'll need to support drag-and-drop patterns as well as drag draw
        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    fn key_down_event(&mut self, opt_keycode: Option<Keycode>, _keymod: Mod, repeat: bool) {
        if opt_keycode == None {
            println!("WARNING: got opt_keycode of None; what could this mean???");
            return;
        }
        let keycode = opt_keycode.unwrap();

        match self.stage {
            Stage::Intro(_) => {
                self.stage = Stage::Run;
            }
            Stage::Menu => {
                // TODO 
            }
            Stage::Run => {
                match keycode {
                    Keycode::Return => {
                        if !repeat {
                            self.running = !self.running;
                        }
                    }
                    Keycode::Space => {
                        self.single_step = true;
                    }
                    Keycode::Up => {
                        self.arrow_input = (0, -1);
                    }
                    Keycode::Down => {
                        self.arrow_input = (0, 1);
                    }
                    Keycode::Left => {
                        self.arrow_input = (-1, 0);
                    }
                    Keycode::Right => {
                        self.arrow_input = (1, 0);
                    }
                    Keycode::Plus | Keycode::Equals => {
                        adjust_zoom_level(self, ZoomDirection::ZoomIn);
                    }
                    Keycode::Minus | Keycode::Underscore => {
                        adjust_zoom_level(self, ZoomDirection::ZoomOut);
                    }
                    Keycode::Num1 => {
                       self.win_resize = 1;
                    }
                    Keycode::Num2 => {
                       self.win_resize = 2;
                    }
                    Keycode::Num3 => {
                       self.win_resize = 3;
                    }
                    Keycode::LGui => {}
                    _ => {
                        println!("Unrecognized keycode {}", keycode);
                    }
                }
            }
        }
    }

    fn key_up_event(&mut self, opt_keycode: Option<Keycode>, _keymod: Mod, _repeat: bool) {
        if opt_keycode == None {
            println!("WARNING: got opt_keycode of None; what could this mean???");
            return;
        }
        let keycode = opt_keycode.unwrap();

        match keycode {
            Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right => {
                self.arrow_input = (0, 0);
            }
            _ => {}
        }
    }

}

fn adjust_zoom_level(main_state: &mut MainState, direction : ZoomDirection) {
    // Zoom In
    if (direction == ZoomDirection::ZoomIn && main_state.grid_view.cell_size < ZOOM_LEVEL_MAX) ||
       (direction == ZoomDirection::ZoomOut && main_state.grid_view.cell_size > ZOOM_LEVEL_MIN) {

        let zoom_dir: i32;
        match direction {
            ZoomDirection::ZoomIn => zoom_dir = 1,
            ZoomDirection::ZoomOut => zoom_dir = -1,
        }

        // TODO Mang Proper logging
        if false {
            println!("Window Size: ({}, {})", main_state.grid_view.rect.width(), main_state.grid_view.rect.height());
            println!("Origin Before: ({},{})", main_state.grid_view.grid_origin.x(), main_state.grid_view.grid_origin.y());
            println!("Cell Size Before: {},", main_state.grid_view.cell_size);
        }

        let old_cell_size = main_state.grid_view.cell_size;
        let next_cell_size = main_state.grid_view.cell_size as i32 + zoom_dir;

        let window_center = Point::new((main_state.grid_view.rect.width()/2) as i32, (main_state.grid_view.rect.height()/2) as i32);

        if let Some((old_cell_count_for_x, old_cell_count_for_y)) = main_state.grid_view.game_coords_from_window(window_center) {
            let delta_x = zoom_dir * (old_cell_count_for_x as i32 * next_cell_size as i32 - old_cell_count_for_x as i32 * old_cell_size as i32);
            let delta_y = zoom_dir * (old_cell_count_for_y as i32 * next_cell_size as i32 - old_cell_count_for_y as i32 * old_cell_size as i32);

            if false {
                println!("current cell count: {}, {}", old_cell_count_for_x, old_cell_count_for_x);
                println!("delta in win coords: {}, {}", delta_x, delta_y);
            }

            main_state.grid_view.cell_size = next_cell_size as u32;

            main_state.grid_view.grid_origin = main_state.grid_view.grid_origin.offset(-zoom_dir * (delta_x as i32), -zoom_dir * (delta_y as i32));

            if false {
                println!("Origin After: ({},{})\n", main_state.grid_view.grid_origin.x(), main_state.grid_view.grid_origin.y());
                println!("Cell Size After: {},", main_state.grid_view.cell_size);
            }
        }
    }
}



// Controls the mapping between window and game coordinates
struct GridView {
    rect:        Rect,  // the area the game grid takes up on screen
    cell_size:   u32,   // zoom level in window coordinates
    columns:     usize, // width in game coords (should match bitmap/universe width)
    rows:        usize, // height in game coords (should match bitmap/universe height)
    // The grid origin point tells us where the top-left of the universe is with respect to the
    // window.
    grid_origin: Point, // top-left corner of grid in window coords. (may be outside rect)
}


impl GridView {
    // Returns Option<(col, row)>
    // Given a Point(x,y), we determine a col/row tuple in cell units
    fn game_coords_from_window(&self, point: Point) -> Option<(usize, usize)> {
/*
        let col: isize = ((point.x() - self.grid_origin.x()) / self.cell_size as i32) as isize;
        let row: isize = ((point.y() - self.grid_origin.y()) / self.cell_size as i32) as isize;
        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some((col as usize, row as usize))
*/
        let col: isize = ((point.x() - self.grid_origin.x()) / self.cell_size as i32) as isize;
        let row: isize = ((point.y() - self.grid_origin.y()) / self.cell_size as i32) as isize;
        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some((col as usize, row as usize))
 
    }

    // Attempt to return a rectangle for the on-screen area of the specified cell.
    // If partially in view, will be clipped by the bounding rectangle.
    // Caller must ensure that col and row are within bounds.
    fn window_coords_from_game(&self, col: usize, row: usize) -> Option<Rect> {
        let left   = self.grid_origin.x() + (col as i32)     * self.cell_size as i32;
        let right  = self.grid_origin.x() + (col + 1) as i32 * self.cell_size as i32 - 1;
        let top    = self.grid_origin.y() + (row as i32)     * self.cell_size as i32;
        let bottom = self.grid_origin.y() + (row + 1) as i32 * self.cell_size as i32 - 1;

        assert!(left < right);
        assert!(top < bottom);
        let rect = Rect::new(left, top, (right - left) as u32, (bottom - top) as u32);
        rect.intersection(self.rect)
    }

    fn grid_width(&self) -> u32 {
        self.columns as u32 * self.cell_size
    }

    fn grid_height(&self) -> u32 {
        self.rows as u32 * self.cell_size
    }
}


// Now our main function, which does three things:
//
// * First, create a new `ggez::conf::Conf`
// object which contains configuration info on things such
// as screen resolution and window title,
// * Second, create a `ggez::game::Game` object which will
// do the work of creating our MainState and running our game,
// * then just call `game.run()` which runs the `Game` mainloop.
pub fn main() {
    let mut c = conf::Conf::new();

    c.version       = version!().to_string();
    c.window_width  = DEFAULT_SCREEN_WIDTH;
    c.window_height = DEFAULT_SCREEN_HEIGHT;
    c.window_icon   = "conwaylife.ico".to_string();
    c.window_title  = "💥 ConWayste the Enemy 💥".to_string();

    // save conf to .toml file
    let mut f = File::create("aaron_conf.toml").unwrap(); //XXX
    c.to_toml_file(&mut f).unwrap();

    let mut game: Game<MainState> = Game::new("ConWaysteTheEnemy", c).unwrap();
    if let Err(e) = game.run() {
        println!("Error encountered: {:?}", e);
    } else {
        println!("Game exited cleanly.");
    }
}
