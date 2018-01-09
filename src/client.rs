/*  Copyright 2017-2018 the Conwayste Developers.
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

extern crate conway;
extern crate env_logger;
extern crate ggez;
#[macro_use] extern crate log;
extern crate sdl2;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate version;

use conway::{BigBang, Universe, CellState, Region, PlayerBuilder};

use ggez::conf;
use ggez::event::*;
use ggez::{GameResult, Context, ContextBuilder};
use ggez::graphics;
use ggez::graphics::{Rect, Point2, Color};
use ggez::timer;

use std::env;
use std::fs::File;
use std::path;
use std::collections::BTreeMap;

mod menu;
mod video;
mod utils;
mod config;
mod viewport;
mod input;

const FPS                       : u32   = 25;
const INTRO_DURATION            : f64   = 2.0;
const HISTORY_SIZE              : usize = 16;
const CURRENT_PLAYER_ID         : usize = 1; // TODO :  get the player ID from server rather than hardcoding
const FOG_RADIUS                : usize = 4;

#[derive(PartialEq, Clone)]
enum Stage {
    Intro(f64),   // seconds
    Menu,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
}

// All game state
struct MainState {
    small_font:          graphics::Font,
    intro_text:          graphics::Text,
    stage:               Stage,             // Where are we in the game (Intro/Menu Main/Running..)
    uni:                 Universe,          // Things alive and moving here
    first_gen_was_drawn: bool,              // The purpose of this is to inhibit gen calc until the first draw
    color_settings:      ColorSettings,
    running:             bool,
    menu_sys:            menu::MenuSystem,
    video_settings:      video::VideoSettings,
    config:              config::ConfigFile,
    viewport:            viewport::Viewport,
    input_manager:       input::InputManager,

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
    for row in 10..19 {
        s.uni.set_unchecked(25, row, CellState::Wall);
    }
    for col in 10..25 {
        s.uni.set_unchecked(col, 10, CellState::Wall);
    }
    for row in 11..23 {
        s.uni.set_unchecked(10, row, CellState::Wall);
    }
    for col in 11..26 {
        s.uni.set_unchecked(col, 22, CellState::Wall);
    }

    Ok(())
}


// Then we implement the `ggez::game::GameState` trait on it, which
// requires callbacks for creating the game state, updating it each
// frame, and drawing it.
//
// The `GameState` trait also contains callbacks for event handling
// that you can override if you wish, but the defaults are fine.
impl MainState {

    fn new(ctx: &mut Context) -> GameResult<MainState> {
        ctx.print_resource_stats();

        let intro_font = graphics::Font::new(ctx, "\\DejaVuSerif.ttf", 32).unwrap();
        let intro_text = graphics::Text::new(ctx, "WAYSTE EM!", &intro_font).unwrap();

        let universe_width_in_cells  = 256;
        let universe_height_in_cells = 120;

        let config = config::ConfigFile::new();

        let mut vs = video::VideoSettings::new();
/*
 *  FIXME Disabling video module temporarily as we can now leverage ggez 0.4
 */
        vs.gather_display_modes(ctx);

        vs.print_resolutions();

/*
        // On first-run, use default supported resolution
        let (w, h) = config.get_resolution();
        if (w,h) != (0,0) {
            vs.set_active_resolution(ctx, w, h);
        } else {
            vs.advance_to_next_resolution(ctx);

            // Some duplication here to update the config file
            // I don't want to do this every load() to avoid
            // unnecessary file writes
            let (w,h) = vs.get_active_resolution();
            config.set_resolution(w,h);
        }
        let (w,h) = vs.get_active_resolution();

        graphics::set_fullscreen(ctx, config.is_fullscreen() == true);
        vs.is_fullscreen = config.is_fullscreen() == true;
*/

        let viewport = viewport::Viewport::new(config.get_zoom_level(), universe_width_in_cells, universe_height_in_cells);

        let mut color_settings = ColorSettings {
            cell_colors: BTreeMap::new(),
            background:  Color::new( 0.25,  0.25,  0.25, 1.0),
        };
        color_settings.cell_colors.insert(CellState::Dead,           Color::new(0.875, 0.875, 0.875, 1.0));
        color_settings.cell_colors.insert(CellState::Alive(None),    Color::new(  0.0,   0.0,   0.0, 1.0));
        color_settings.cell_colors.insert(CellState::Alive(Some(0)), Color::new(  1.0,   0.0,   0.0, 1.0));  // 0 is red
        color_settings.cell_colors.insert(CellState::Alive(Some(1)), Color::new(  0.0,   0.0,   1.0, 1.0));  // 1 is blue
        color_settings.cell_colors.insert(CellState::Wall,           Color::new(0.617,  0.55,  0.41, 1.0));
        color_settings.cell_colors.insert(CellState::Fog,            Color::new(0.780, 0.780, 0.780, 1.0));

        let small_font = graphics::Font::new(ctx, "\\DejaVuSerif.ttf", 20).unwrap();
        let menu_font  = graphics::Font::new(ctx, "\\DejaVuSerif.ttf", 20).unwrap();

        let bigbang = 
        {
            // we're going to have to tear this all out when this becomes a real game
            let player0_writable = Region::new(100, 70, 34, 16);
            let player1_writable = Region::new(0, 0, 80, 80);

            let player0 = PlayerBuilder::new(player0_writable);
            let player1 = PlayerBuilder::new(player1_writable);
            let players = vec![player0, player1];

            BigBang::new()
            .width(universe_width_in_cells)
            .height(universe_height_in_cells)
            .server_mode(true) // TODO will change to false once we get server support up
                               // Currently 'client' is technically both client and server
            .history(HISTORY_SIZE)
            .fog_radius(FOG_RADIUS)
            .add_players(players)
            .birth()
        };

        let mut s = MainState {
            small_font:          small_font,
            intro_text:          intro_text,
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 bigbang.unwrap(),
            first_gen_was_drawn: false,
            color_settings:      color_settings,
            running:             false,
            menu_sys:            menu::MenuSystem::new(menu_font),
            video_settings:      vs,
            config:              config,
            viewport:            viewport,
            input_manager:       input::InputManager::new(input::InputDeviceType::PRIMARY),
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            win_resize:          0,
            return_key_pressed:  false,
            escape_key_pressed:  false,
            toggle_paused_game:  false,
        };

        init_patterns(&mut s).unwrap();

        Ok(s)
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let duration = timer::duration_to_f64(timer::get_delta(ctx)); // seconds

        let cur_menu_state = {
            self.menu_sys.menu_state.clone()
        };

        match self.stage {
            Stage::Intro(mut remaining) => {
                remaining -= duration;
                if remaining > 0.0 {
                    self.stage = Stage::Intro(remaining);
                } else {
                    self.stage = Stage::Run; // Menu Stage is disabled for the time being
                    self.menu_sys.menu_state = menu::MenuState::MainMenu;
                }
            }
            Stage::Menu => {
                if self.config.is_dirty() {
                    self.config.write();
                }

                self.process_menu_inputs();

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
                        let mainmenu_md = container.get_metadata();
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
                                            self.video_settings.is_fullscreen = video::toggle_full_screen(ctx);
                                            self.config.set_fullscreen(self.video_settings.is_fullscreen == true);
                                        }
                                    }
                                    menu::MenuItemIdentifier::Resolution => {
                                        if !self.escape_key_pressed {
                                            self.video_settings.advance_to_next_resolution(ctx);

                                            // Update the configuration file and resize the viewing
                                            // screen
                                            let (w,h) = self.video_settings.get_active_resolution();
                                            self.config.set_resolution(w as i32, h as i32);
                                            self.viewport.set_dimensions(w, h);
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
// TODO while this works at limiting the FPS, its a bit glitchy for input events
// Disable until we have time to look into it
//                while timer::check_update_time(ctx, FPS) { 
                {
                    if self.single_step {
                        self.running = false;
                    }

                    self.process_running_inputs();

                    if self.first_gen_was_drawn && (self.running || self.single_step) {
                        self.uni.next();     // next generation
                        self.single_step = false;
                    }

                    if self.toggle_paused_game {
                        self.pause_or_resume_game();
                    }

                    self.viewport.update(self.arrow_input); // TODO needs input reference
                }
            }
            Stage::Exit => {
               let _ = ctx.quit();
            }
        }

        self.input_manager.expunge();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);
        graphics::set_background_color(ctx, (0, 0, 0, 1).into());

        match self.stage {
            Stage::Intro(_) => {
                graphics::draw(ctx, &mut self.intro_text, Point2::new(0.0, 0.0), 0.0)?;
            }
            Stage::Menu => {
                self.menu_sys.draw_menu(&self.video_settings, ctx, self.first_gen_was_drawn);
            }
            Stage::Run => {
                self.draw_universe(ctx)?;
            }
            Stage::Exit => {}
        }

        graphics::present(ctx);
        timer::yield_now();
        Ok(())
    }

    fn mouse_button_down_event(&mut self,
                               ctx: &mut Context,
                               button: MouseButton,
                               x: i32,
                               y: i32
                               ) {
        self.input_manager.add(input::InputAction::MouseClick(button, x, y));
    }

    fn mouse_motion_event(&mut self,
                          ctx: &mut Context,
                          state: MouseState,
                          x: i32,
                          y: i32,
                          _xrel: i32,
                          _yrel: i32
                          ) {
        match self.stage {
            Stage::Intro(_) => {}
            Stage::Menu | Stage::Run => {
                if state.left() && self.drag_draw != None {
                    self.input_manager.add(input::InputAction::MouseDrag(MouseButton::Left, x, y));
                } else {
                    self.input_manager.add(input::InputAction::MouseMovement(x, y));
                }
            }
            Stage::Exit => {}
        }
    }

    fn mouse_button_up_event(&mut self,
                             ctx: &mut Context,
                             _button: MouseButton,
                             _x: i32,
                             _y: i32
                             ) {
        // TODO Later, we'll need to support drag-and-drop patterns as well as drag draw
        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    fn key_down_event(&mut self,
                      ctx: &mut Context,
                      keycode: Keycode,
                      _keymod: Mod,
                      repeat: bool
                      ) {

        match self.stage {
            Stage::Intro(_) => {
                self.stage = Stage::Menu;
            }
            Stage::Menu | Stage::Run => {
                // TODO for now just quit the game
                if keycode == Keycode::Escape {
                    self.quit_event(ctx);
                }
                self.input_manager.add(input::InputAction::KeyPress(keycode, repeat));
            }
            Stage::Exit => {}
        }
    }

    fn key_up_event(&mut self,
                    ctx: &mut Context,
                    keycode: Keycode,
                    _keymod: Mod,
                    _repeat: bool
                    ) {
        self.input_manager.add(input::InputAction::KeyRelease(keycode));
    }

    fn quit_event(&mut self, ctx: &mut Context) -> bool {
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
            _ => {}
        }

        do_not_quit
    }

}

impl MainState {
    fn draw_universe(&mut self, ctx: &mut Context) -> GameResult<()> {
        // grid background
        graphics::set_color(ctx, self.color_settings.get_color(None))?;
        graphics::rectangle(ctx,  graphics::DrawMode::Fill, self.viewport.get_viewport())?;

        // grid foreground (dead cells)
        let origin = self.viewport.get_origin();
        let full_width  = self.viewport.grid_width() as f32;
        let full_height = self.viewport.grid_height() as f32;

        let full_rect = Rect::new(origin.x, origin.y, full_width, full_height);

        if let Some(clipped_rect) = utils::Graphics::intersection(full_rect, self.viewport.get_viewport()) {
            graphics::set_color(ctx, self.color_settings.get_color(Some(CellState::Dead)))?;
            graphics::rectangle(ctx,  graphics::DrawMode::Fill, clipped_rect)?;
        }

        // grid non-dead cells (walls, players, etc.)
        let visibility = Some(1); //XXX, Player One

        let image = graphics::Image::solid(ctx, 1u16, Color::new(1.0, 1.0, 1.0, 1.0))?; // 1x1 square
        let mut spritebatch = graphics::spritebatch::SpriteBatch::new(image);

        self.uni.each_non_dead_full(visibility, &mut |col, row, state| {
            let color = self.color_settings.get_color(Some(state));
            let _ = graphics::set_color(ctx, color);

            if let Some(rect) = self.viewport.get_screen_area(col, row) {
                let p = graphics::DrawParam {
                    dest: Point2::new(rect.x, rect.y),
                    scale: Point2::new(rect.w, rect.h), // scaling a 1x1 Image to correct cell size
                    color: Some(color),
                    ..Default::default()
                };

                spritebatch.add(p);
            }
        });

        ////////// draw generation counter
        let gen_counter_str = self.uni.latest_gen().to_string();
        graphics::set_color(ctx, Color::new(1.0, 0.0, 0.0, 1.0))?;
        utils::Graphics::draw_text(ctx, &self.small_font, &gen_counter_str, &Point2::new(0.0, 0.0), None);

        ////////////////////// END
        graphics::set_color(ctx, Color::new(0.0, 0.0, 0.0, 1.0))?; // Clear color residue
        self.first_gen_was_drawn = true;

        graphics::draw_ex(ctx, &spritebatch, graphics::DrawParam{ dest: Point2::new(0.0, 0.0), .. Default::default()} )?;
        spritebatch.clear();
        Ok(())
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
                if cur_menu_state == menu::MenuState::MenuOff {
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

    // TODO
    // Just had an idea... transform the user inputs events into game events
    // That way differnt user inputs (gamepad, keyboard/mouse) resolve to a single game event
    // This logic would be agnostic from how the user is interacting with the game
    fn process_running_inputs(&mut self) {
        while self.input_manager.has_more() {
            if let Some(input) = self.input_manager.remove() {
                match input {

                    // MOUSE EVENTS
                    input::InputAction::MouseClick(MouseButton::Left, x, y) => {
                        // Need to go through UI manager to determine what we are interacting with, TODO
                        // Could be UI element (drawpad) or playing field
                        if let Some(cell) = self.viewport.get_cell(Point2::new(x as f32, y as f32)) {
                            let result = self.uni.toggle(cell.col, cell.row, CURRENT_PLAYER_ID);
                            self.drag_draw = match result {
                                Ok(state) => Some(state),
                                Err(_)    => None,
                            };
                        }
                    }
                    input::InputAction::MouseClick(MouseButton::Right, _x, _y) => { }
                    input::InputAction::MouseMovement(_x, _y) => { }
                    input::InputAction::MouseDrag(MouseButton::Left, x, y) => {
                        if let Some(cell) = self.viewport.get_cell(Point2::new(x as f32, y as f32)) {
                            if let Some(cell_state) = self.drag_draw {
                                self.uni.set(cell.col, cell.row, cell_state, CURRENT_PLAYER_ID);
                            }
                        }
                    }

                    // KEYBOARD EVENTS
                    input::InputAction::KeyPress(keycode, repeat) => {
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
                                self.viewport.adjust_zoom_level(viewport::ZoomDirection::ZoomIn);
                                self.config.set_zoom_level(self.viewport.get_cell_size());
                            }
                            Keycode::Minus | Keycode::Underscore => {
                                self.viewport.adjust_zoom_level(viewport::ZoomDirection::ZoomOut);
                                self.config.set_zoom_level(self.viewport.get_cell_size());
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
                            Keycode::P => {
                                self.config.print_to_screen();
                            }
                            Keycode::LGui => {
                            
                            }
                            _ => {
                                println!("Unrecognized keycode {}", keycode);
                            }
                        }
                    }

                    _ => {},
                }
            }
        }
    }

    fn process_menu_inputs(&mut self) {
        while self.input_manager.has_more() {
            if let Some(input) = self.input_manager.remove() {
                match input {
                    input::InputAction::MouseClick(MouseButton::Left, _x, _y) => {}
                    input::InputAction::MouseClick(MouseButton::Right, _x, _y) => {}
                    input::InputAction::MouseMovement(x, y) => {}
                    input::InputAction::MouseDrag(MouseButton::Left, _x, _y) => {}
                    input::InputAction::MouseRelease(_) => {}

                    input::InputAction::KeyPress(keycode, repeat) => {
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
                                    if !repeat {
                                        self.return_key_pressed = true;
                                    }
                                }
                                Keycode::Escape => {
                                    self.escape_key_pressed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    input::InputAction::KeyRelease(keycode) => {
                        match keycode {
                            Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right => {
                                self.arrow_input = (0, 0);
                                self.menu_sys.get_controls().set_menu_key_pressed(false);
                            }
                            _ => {}
                        }
                    }

                    _ => {},
                }
            }
        }

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

    let mut cb = ContextBuilder::new("conwayste", "Aaronm04Manghi")
        .window_setup(conf::WindowSetup::default()
                      .title(format!("{} {} {}", "💥 conwayste", version!().to_owned(),"💥").as_str())
                      .icon("\\conwayste.ico")
                      .resizable(false)
                      .allow_highdpi(true)
                      )
        .window_mode(conf::WindowMode::default()
                     .dimensions(config::DEFAULT_SCREEN_WIDTH as u32, config::DEFAULT_SCREEN_HEIGHT as u32)
                     .vsync(true)
                     );

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {:?}", path);
        cb = cb.add_resource_path(path);
    } else {
        println!("Not building from cargo? Okie dokie.");
    }

    let ctx = &mut cb.build().unwrap();

    match MainState::new(ctx) {
        Err(e) => {
            println!("Could not load Conwayste!");
            println!("Error: {}", e);
        }
        Ok(ref mut game) => {
            let result = run(ctx, game);
            if let Err(e) = result {
                println!("Error encountered while running game: {}", e);
            } else {
                println!("Game exited cleanly.");
            }
        }
    }
}
