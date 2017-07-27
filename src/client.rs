
/*  Copyright 2017 the Conwayste Developers.
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

// TODOs
// :) detect screen resolution native
// :) current default resolution
// :) full screen/ window toggle support
// :) main menu & settings
// unit tests
// :) logging support
// :) align panning to window border
// 
// Modularization
// :) Menu System
//

extern crate conway;
extern crate ggez;
#[macro_use]
extern crate version;
extern crate sdl2;
#[macro_use] extern crate log;
extern crate env_logger;

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

mod menu;
mod video;
mod utils;


const FPS: u32 = 25;
const INTRO_DURATION: f64 = 2.0;
const DEFAULT_SCREEN_WIDTH: u32 = 1200;
const DEFAULT_SCREEN_HEIGHT: u32 = 800;
const PIXELS_SCROLLED_PER_FRAME: i32 = 50;
const ZOOM_LEVEL_MIN: u32 = 4;
const ZOOM_LEVEL_MAX: u32 = 20;
const HISTORY_SIZE: usize = 16;
const NUM_PLAYERS: usize = 2;

#[derive(PartialEq, Clone)]
enum Stage {
    Intro(f64),   // seconds
    Menu,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
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
    menu_sys:            menu::MenuSystem,
    video_settings:      video::VideoSettings,

    // Input state
    single_step:         bool,
    arrow_input:         (i32, i32),
    drag_draw:           Option<CellState>,
    win_resize:          u8,
    return_key_pressed:  bool,
    escape_key_pressed:  bool,
    toggle_paused_game:  bool,
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

    fn load(_ctx: &mut Context, _conf: &conf::Conf) -> GameResult<MainState> {
        let intro_font = graphics::Font::new(_ctx, "DejaVuSerif.ttf", 32).unwrap();
        let intro_text = graphics::Text::new(_ctx, "WAYSTE EM!", &intro_font).unwrap();

        let universe_width_in_cells  = 256;
        let universe_height_in_cells = 120;

        let mut vs = video::VideoSettings::new();
        vs.gather_display_modes(_ctx);
        vs.print_resolutions();
        // This will set resolution to first supported & discovered res
        vs.advance_to_next_resolution(_ctx);
        let (w,h) = vs.get_active_resolution();

        let grid_view = GridView {
            rect:        Rect::new(0, 0, w as u32, h as u32),
            cell_size:   10,
            columns:     universe_width_in_cells,
            rows:        universe_height_in_cells,
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
        color_settings.cell_colors.insert(CellState::Fog,            Color::RGB(200, 200, 200));

        // we're going to have to tear this all out when this becomes a real game
        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];

        let small_font = graphics::Font::new(_ctx, "DejaVuSerif.ttf", 20).unwrap();
        let menu_font = graphics::Font::new(_ctx, "DejaVuSerif.ttf", 20).unwrap();
        let mut s = MainState {
            small_font:          small_font,
            intro_text:          intro_text,
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 Universe::new(universe_width_in_cells, universe_height_in_cells, true, HISTORY_SIZE, NUM_PLAYERS, writable_regions).unwrap(),
            first_gen_was_drawn: false,
            grid_view:           grid_view,
            color_settings:      color_settings,
            running:             false,
            menu_sys:            menu::MenuSystem::new(menu_font),
            video_settings:      vs,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            win_resize:          0,
            return_key_pressed:  false,
            escape_key_pressed:  false, // Action flag, available for use
            toggle_paused_game:  false,
        };

        init_patterns(&mut s).unwrap();

        Ok(s)
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) -> GameResult<()> {
        let duration = timer::duration_to_f64(dt); // seconds

        let cur_menu_state = {
            self.menu_sys.menu_state.clone()
        };

        match self.stage {
            Stage::Intro(mut remaining) => {
                remaining -= duration;
                if remaining > 0.0 {
                    self.stage = Stage::Intro(remaining);
                } else {
                    self.stage = Stage::Menu;
                    self.menu_sys.menu_state = menu::MenuState::MainMenu;
                }
            }
            Stage::Menu => {
                
                let is_direction_key_pressed = {
                    self.menu_sys.get_controls().is_menu_key_pressed()
                };

                //// Directional Key / Menu movement
                ////////////////////////////////////////
                if self.arrow_input != (0,0) && !is_direction_key_pressed {
                    // move selection accordingly
                    let (_,y) = self.arrow_input;
                    {
                        let container = self.menu_sys.get_menu_container(&cur_menu_state); 
                        let mut mainmenu_md = container.get_metadata();
                        mainmenu_md.adjust_index(y);
                    }
                    self.menu_sys.get_controls().set_menu_key_pressed(true);
                }
                else {
                    /////////////////////////
                    //// Enter key was pressed
                    //////////////////////////

                    if self.return_key_pressed || self.escape_key_pressed {

                        let mut id = {
                            let container = self.menu_sys.get_menu_container(&cur_menu_state);
                            let index = container.get_metadata().get_index();
                            let menu_item_list = container.get_menu_item_list();
                            let menu_item = menu_item_list.get(index).unwrap();
                            menu_item.get_id()
                        };

                        if self.escape_key_pressed {
                            id = menu::MenuItemIdentifier::ReturnToPreviousMenu;
                        }

                        match cur_menu_state {
                            menu::MenuState::MainMenu => {
                                if !self.escape_key_pressed {
                                    match id {
                                        menu::MenuItemIdentifier::StartGame => {
                                            self.pause_or_resume_game();
                                        }
                                        menu::MenuItemIdentifier::ExitGame => {
                                            self.stage = Stage::Exit;
                                        }
                                        menu::MenuItemIdentifier::Options => {
                                            self.menu_sys.menu_state = menu::MenuState::Options;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            menu::MenuState::Options => {
                                match id {
                                    menu::MenuItemIdentifier::VideoSettings => {
                                        if !self.escape_key_pressed {
                                            self.menu_sys.menu_state = menu::MenuState::Video;
                                        }
                                    }
                                    menu::MenuItemIdentifier::AudioSettings => {
                                        if !self.escape_key_pressed {
                                            self.menu_sys.menu_state = menu::MenuState::Audio;
                                        }
                                    }
                                    menu::MenuItemIdentifier::GameplaySettings => {
                                        if !self.escape_key_pressed {
                                            self.menu_sys.menu_state = menu::MenuState::Gameplay;
                                        }
                                    }
                                    menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                            self.menu_sys.menu_state = menu::MenuState::MainMenu;
                                    }
                                   _ => {}
                                }
                            }
                            menu::MenuState::MenuOff => {

                            }
                            menu::MenuState::Audio => {
                                match id {
                                    menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                        self.menu_sys.menu_state = menu::MenuState::Options;
                                    }
                                    _ => {
                                        if !self.escape_key_pressed { }
                                    }
                                }
                            }
                            menu::MenuState::Gameplay => {
                                match id {
                                    menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                        self.menu_sys.menu_state = menu::MenuState::Options;
                                    }
                                    _ => {
                                        if !self.escape_key_pressed { }
                                    }
                                }
                            }
                            menu::MenuState::Video => {
                                match id {
                                    menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                        self.menu_sys.menu_state = menu::MenuState::Options;
                                    }
                                    menu::MenuItemIdentifier::Fullscreen => {
                                        if !self.escape_key_pressed {
                                            self.video_settings.is_fullscreen = video::toggle_full_screen(_ctx);
                                        }
                                    }
                                    menu::MenuItemIdentifier::Resolution => {
                                        if !self.escape_key_pressed {
                                            self.video_settings.advance_to_next_resolution(_ctx);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                self.return_key_pressed = false;
                self.escape_key_pressed = false;
            }
            Stage::Run => {
                if self.single_step {
                    self.running = false;
                }

                if self.first_gen_was_drawn && (self.running || self.single_step) {
                    self.uni.next();     // next generation
                    self.single_step = false;
                }

                if self.toggle_paused_game {
                    self.pause_or_resume_game();
                }

                if self.arrow_input != (0, 0) {
                    self.adjust_panning();
                }
            }
            Stage::Exit => {
               let _ = _ctx.quit();
            }
        }

        Ok(())
    }

    fn draw(&mut self, _ctx: &mut Context) -> GameResult<()> {
       _ctx.renderer.clear();
        match self.stage {
            Stage::Intro(_) => {
                try!(graphics::draw(_ctx, &mut self.intro_text, None, None));
            }
            Stage::Menu => {
                self.menu_sys.draw_menu(&self.video_settings, _ctx, self.first_gen_was_drawn);
            }
            Stage::Run => {
                ////////// draw universe
                self.draw_universe(_ctx);
            }
            Stage::Exit => {

            }
        }

       _ctx.renderer.present();
        timer::sleep_until_next_frame(_ctx, FPS);
        Ok(())
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: i32, y: i32) {
        match self.stage {
            Stage::Run => {
                if button == MouseButton::Left {
                    if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                        let result = self.uni.toggle(col as usize, row  as usize, 1);   // TODO: don't hardcode the player number
                        self.drag_draw = match result {
                            Ok(state) => Some(state),
                            Err(_)    => None,
                        };
                    }
                }
            }
            _ => {}
        }
    }

    fn mouse_motion_event(&mut self, state: MouseState, x: i32, y: i32, _xrel: i32, _yrel: i32) {
        if state.left() && self.drag_draw != None {
            if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                let cell_state = self.drag_draw.unwrap();
                self.uni.set(col as usize, row  as usize, cell_state);
            }
        }
    }

    fn mouse_button_up_event(&mut self, _button: MouseButton, _x: i32, _y: i32) {
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
                self.stage = Stage::Menu;
            }
            Stage::Menu => {
                
                if !self.menu_sys.get_controls().is_menu_key_pressed() {
                    match keycode {
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
                        Keycode::Return => {
                            self.return_key_pressed = true;
                        }
                        Keycode::Escape => {
                            self.escape_key_pressed = true;
                        }
                        _ => {

                        }
                    }
                }
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
                        self.adjust_zoom_level(ZoomDirection::ZoomIn);
                    }
                    Keycode::Minus | Keycode::Underscore => {
                        self.adjust_zoom_level(ZoomDirection::ZoomOut);
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
                    Keycode::LGui => {
                    
                    }
                    _ => {
                        println!("Unrecognized keycode {}", keycode);
                    }
                }
            }
            Stage::Exit => {

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
                self.menu_sys.get_controls().set_menu_key_pressed(false);
            }
            _ => {}
        }
    }

    fn quit_event(&mut self) -> bool {
        let mut do_not_quit = true;

        match self.stage {
            Stage::Run => {
                self.pause_or_resume_game();
            }
            Stage::Menu => {
                // This is currently handled in the return_key_pressed path as well
                self.escape_key_pressed = true;
            }
            Stage::Exit => {
                do_not_quit = false;
            }
            _ => {

            }
        }

        do_not_quit
    }

}

impl MainState {
    fn draw_universe(&mut self, _ctx: &mut Context) {
        // grid background
        graphics::set_color(_ctx, self.color_settings.get_color(None));
        graphics::rectangle(_ctx,  graphics::DrawMode::Fill, self.grid_view.rect).unwrap();

        // grid foreground (dead cells)
        //TODO: put in its own function (of GridView); also make this less ugly
        let origin = self.grid_view.grid_origin;
        let full_width  = self.grid_view.grid_width();
        let full_height = self.grid_view.grid_height();
        let full_rect = Rect::new(origin.x(), origin.y(), full_width, full_height);

        if let Some(clipped_rect) = full_rect.intersection(self.grid_view.rect) {
            graphics::set_color(_ctx, self.color_settings.get_color(Some(CellState::Dead)));
            graphics::rectangle(_ctx,  graphics::DrawMode::Fill, clipped_rect).unwrap();
        }

        // grid non-dead cells
        let visibility = Some(1); //XXX
        self.uni.each_non_dead_full(visibility, &mut |col, row, state| {
            let color = self.color_settings.get_color(Some(state));
            graphics::set_color(_ctx, color);

            if let Some(rect) = self.grid_view.window_coords_from_game(col, row) {
                graphics::rectangle(_ctx,  graphics::DrawMode::Fill, rect).unwrap();
            }
        });

        ////////// draw generation counter
        let gen_counter_str = self.uni.latest_gen().to_string();
        utils::Graphics::draw_text(_ctx, &self.small_font, &gen_counter_str, &Point::new(0,0), None);

        ////////////////////// END
        graphics::set_color(_ctx, Color::RGB(0,0,0)); // do this at end; not sure why...?
        self.first_gen_was_drawn = true;

    }

    fn pause_or_resume_game(&mut self) {
        let cur_menu_state = {
            self.menu_sys.menu_state.clone()
        };
        let cur_stage = {
            self.stage.clone()
        };

        match cur_stage {
            Stage::Menu => {
                if cur_menu_state == menu::MenuState::MainMenu {
                    self.stage = Stage::Run;
                    self.menu_sys.menu_state = menu::MenuState::MenuOff;
                    self.running = true;
                }
            }
            Stage::Run => {
                if menu::MenuState::MenuOff == cur_menu_state {
                    self.stage = Stage::Menu;
                    self.menu_sys.menu_state = menu::MenuState::MainMenu;
                    self.running = false;
                }
                else {
                    panic!("Menu State should be OFF while game is in progress: {:?}", cur_menu_state);
                }
            }
            _ => unimplemented!()
        }

        self.toggle_paused_game = false;
    }

    fn adjust_zoom_level(&mut self, direction : ZoomDirection) {
        if (direction == ZoomDirection::ZoomIn && self.grid_view.cell_size < ZOOM_LEVEL_MAX) ||
           (direction == ZoomDirection::ZoomOut && self.grid_view.cell_size > ZOOM_LEVEL_MIN) {

            let zoom_dir: i32;
            match direction {
                ZoomDirection::ZoomIn => zoom_dir = 1,
                ZoomDirection::ZoomOut => zoom_dir = -1,
            }

            debug!("Window Size: ({}, {})", self.grid_view.rect.width(), self.grid_view.rect.height());
            debug!("Origin Before: ({},{})", self.grid_view.grid_origin.x(), self.grid_view.grid_origin.y());
            debug!("Cell Size Before: {},", self.grid_view.cell_size);

            let old_cell_size = self.grid_view.cell_size;
            let next_cell_size = self.grid_view.cell_size as i32 + zoom_dir;

            let window_center = Point::new((self.grid_view.rect.width()/2) as i32, (self.grid_view.rect.height()/2) as i32);

            if let Some((old_cell_count_for_x, old_cell_count_for_y)) = self.grid_view.game_coords_from_window(window_center) {
                let delta_x = zoom_dir * (old_cell_count_for_x as i32 * next_cell_size as i32 - old_cell_count_for_x as i32 * old_cell_size as i32);
                let delta_y = zoom_dir * (old_cell_count_for_y as i32 * next_cell_size as i32 - old_cell_count_for_y as i32 * old_cell_size as i32);

                debug!("current cell count: {}, {}", old_cell_count_for_x, old_cell_count_for_x);
                debug!("delta in win coords: {}, {}", delta_x, delta_y);

                self.grid_view.cell_size = next_cell_size as u32;

                self.grid_view.grid_origin = self.grid_view.grid_origin.offset(-zoom_dir * (delta_x as i32), -zoom_dir * (delta_y as i32));

                debug!("Origin After: ({},{})\n", self.grid_view.grid_origin.x(), self.grid_view.grid_origin.y());
                debug!("Cell Size After: {},", self.grid_view.cell_size);
            }
        }
    }

    fn adjust_panning(&mut self) {
        let cell_size = self.grid_view.cell_size;
        let (columns, rows) = (self.grid_view.columns as u32, self.grid_view.rows as u32);

        // When qualifying a new origin for movement, we need to always keep 
        // in mind where it, and the bottom/right corner, stand with respect to the boundaries of the virtual window edges.
        //
        //   Universe
        // ---------------               
        // |                             
        // |     Window      Window      
        // |   +====                :   |
        // |   :                    :   |
        // |   :                ====+   |
        // |                            |
        // |                -------------
        //                     Universe  
        //

        debug!("\n\nP A N N I N G:");
        debug!("Columns, Rows = {:?}", (columns, rows));

        let (dx, dy) = self.arrow_input;
        let dx_in_pixels = -dx * PIXELS_SCROLLED_PER_FRAME;
        let dy_in_pixels = -dy * PIXELS_SCROLLED_PER_FRAME;

        let cur_origin_x = self.grid_view.grid_origin.x();
        let cur_origin_y = self.grid_view.grid_origin.y();

        let mut new_origin_x = cur_origin_x + dx_in_pixels;
        let mut new_origin_y = cur_origin_y + dy_in_pixels;

        let cur_origin_x_in_gc = cur_origin_x/cell_size as i32 + 1;
        let cur_origin_y_in_gc = cur_origin_y/cell_size as i32 + 1;

        let new_origin_x_in_gc = new_origin_x/cell_size as i32 + 1;
        let new_origin_y_in_gc = new_origin_y/cell_size as i32 + 1;

        let border_in_cells = 10;

        debug!("Cell Size: {:?}", (cell_size));

        let right_boundary_in_gc = columns as i32;
        let bottom_boundary_in_gc = rows  as i32;

        debug!("Cur Origin in GC: {:?}", (cur_origin_x_in_gc, cur_origin_y_in_gc));
        debug!("Bottom Right Corner in GC: {:?}", (right_boundary_in_gc, bottom_boundary_in_gc));

        // Get the game coordinates of the two corners
        if let Some(w_in_gc) = self.get_all_window_coords_in_game_coords() { // use w_in_gc to calculate the distance between
            let (mut offset_x, mut offset_y) = (dx_in_pixels, dy_in_pixels);
            let mut pan_x = true;
            let mut pan_y = true;

            debug!("[X] Top Left Window::{:?}  |  TL Universe::{:?}", w_in_gc.top_left.x(), new_origin_x_in_gc);
            if (w_in_gc.top_left.x() < new_origin_x_in_gc)
            && (new_origin_x_in_gc - w_in_gc.top_left.x() > border_in_cells) {
                offset_x = -10;
                pan_x = true;
                debug!("Could not pan [Top_Left_X]");
            }

             debug!("[Y] Top Left Window::{:?}  |  TL Universe::{:?}", w_in_gc.top_left.y(), new_origin_y_in_gc);
            if (w_in_gc.top_left.y() < new_origin_y_in_gc)
            && (new_origin_y_in_gc - w_in_gc.top_left.y() > border_in_cells) {
                offset_y = -10;
                pan_y = true;
                debug!("Could not pan [Top_Left_Y]");
            }
            
            debug!("[X] Bot Right Window::{:?}  |  BR Universe::{:?}", w_in_gc.bottom_right.x(), right_boundary_in_gc);
            if (w_in_gc.bottom_right.x() > right_boundary_in_gc)
            && (w_in_gc.bottom_right.x() - right_boundary_in_gc > border_in_cells) {
                offset_x = 10;
                pan_x = true;
                debug!("Could not pan [Bot_Right_X]");
            }

            debug!("[Y] Bot Right Window::{:?}  |  BR Universe::{:?}", w_in_gc.bottom_right.y(), bottom_boundary_in_gc);
            if (w_in_gc.bottom_right.y() > bottom_boundary_in_gc)
            && (w_in_gc.bottom_right.y() - bottom_boundary_in_gc > border_in_cells) {
                offset_y = 10;
                pan_y = true;
                debug!("Could not pan [Bot_Right_Y]");
            }
            
            debug!("TwoCanPan [X|Y]: {:?}", (pan_x, pan_y));
            debug!("Panning Offsets: {:?}", (offset_x, offset_y));
            
            if pan_x {
                self.grid_view.grid_origin = self.grid_view.grid_origin.offset(offset_x, 0);
            }

            if pan_y {
                self.grid_view.grid_origin = self.grid_view.grid_origin.offset(0, offset_y);
            }

            if pan_y || pan_x {
                debug!("New Origin: {:?}", self.grid_view.grid_origin);
            }
        }

        new_origin_x = self.grid_view.grid_origin.x();
        new_origin_y = self.grid_view.grid_origin.y();

        debug!("New Origin: {:?}", (new_origin_x, new_origin_y));
        debug!("Bottom Right Corner In Cells: {:?}", (right_boundary_in_gc, bottom_boundary_in_gc));
    }

     fn get_all_window_coords_in_game_coords(&mut self) -> Option<WindowCornersInGameCoords> {
        let resolution = self.video_settings.get_active_resolution();
        let win_width_px = resolution.0 as i32;
        let win_height_px = resolution.1 as i32;

        debug!("\tWindow: {:?} px", (win_width_px, win_height_px));

        let result : Option<WindowCornersInGameCoords>;

        let (origin_x, origin_y) = self.grid_view.game_coords_from_window_unchecked(Point::new(0, 0));
        {
            let (win_width_px, win_height_px) = self.grid_view.game_coords_from_window_unchecked(Point::new(win_width_px, win_height_px));
            {
                result = Some(WindowCornersInGameCoords {
                    top_left : Point::new(origin_x as i32, origin_y as i32),
                    bottom_right : Point::new(win_width_px as i32, win_height_px as i32),
                });
                debug!("\tReturning... {:?}", result);
            }
        }

        result
    }
}


#[derive(Debug, Clone)]
struct WindowCornersInGameCoords {
    top_left : Point,
    bottom_right: Point,
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



    fn game_coords_from_window_unchecked(&self, point: Point) -> (isize, isize) {
        let col: isize = ((point.x() - self.grid_origin.x()) / self.cell_size as i32) as isize;
        let row: isize = ((point.y() - self.grid_origin.y()) / self.cell_size as i32) as isize;
        
        (col , row )
    }

    // Given a Point(x,y), we determine a col/row tuple in cell units
    fn game_coords_from_window(&self, point: Point) -> Option<(isize, isize)> {
        let (col, row) = self.game_coords_from_window_unchecked(point);

        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some((col , row ))
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
    env_logger::init().unwrap();
    let mut c = conf::Conf::new();

    c.version       = version!().to_string();
    c.window_width  = DEFAULT_SCREEN_WIDTH;
    c.window_height = DEFAULT_SCREEN_HEIGHT;
    c.window_icon   = "conwayste.ico".to_string();
    c.window_title  = "💥 conwayste 💥".to_string();

    // save conf to .toml file
    let mut f = File::create("conwayste.toml").unwrap(); //XXX
    c.to_toml_file(&mut f).unwrap();

    let mut game: Game<MainState> = Game::new("conwayste", c).unwrap();
    if let Err(e) = game.run() {
        println!("Error encountered: {:?}", e);
    } else {
        println!("Game exited cleanly.");
    }
}
