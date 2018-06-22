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
extern crate rand;

mod config;
mod constants;
mod input;
mod menu;
mod utils;
mod video;
mod viewport;

use conway::{BigBang, Universe, CellState, Region, PlayerBuilder};

use ggez::conf;
use ggez::event::*;
use ggez::{GameResult, Context, ContextBuilder};
use ggez::graphics;
use ggez::graphics::{Point2, Color};
use ggez::timer;

use std::env;
use std::path;
use std::collections::BTreeMap;

use constants::{
    CURRENT_PLAYER_ID,
    DEFAULT_SCREEN_HEIGHT,
    DEFAULT_SCREEN_WIDTH,
    DEFAULT_ZOOM_LEVEL,
    DrawStyle,
    FOG_RADIUS,
    GRID_DRAW_STYLE,
    HISTORY_SIZE,
    INTRO_DURATION,
    INTRO_PAUSE_DURATION,
};

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    Intro(f64),   // seconds
    Menu,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
}

// All game state
struct MainState {
    small_font:          graphics::Font,
    screen:              Screen,            // Where are we in the game (Intro/Menu Main/Running..)
    uni:                 Universe,          // Things alive and moving here
    intro_uni:           Universe,
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
    arrow_input:         (isize, isize),
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

    fn get_random_color(&self) -> Color {
        use rand::distributions::{IndependentSample, Range};
        let range = Range::new(0.0, 1.0);
        let mut colors = vec![1.0, 2.0, 3.0];
        let mut rng = rand::thread_rng();

        for x in colors.iter_mut() {
            *x = range.ind_sample(&mut rng);
        }
        let mut iter = colors.into_iter();
        Color::new(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), 1.0)

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


enum Orientation {
    Vertical,
    Horizontal,
    Diagonal
}

// Toggle a horizontal, vertical, or diagonal line, as player with index 0. This is only used for
// the intro currently. Part or all of the line can be outside of the Universe; if this is the
// case, only the parts inside the Universe are toggled.
fn toggle_line(s: &mut MainState, orientation: Orientation, col: isize, row: isize, width: isize, height: isize) {
    let player_id = 0;   // hardcode player ID, since this is just for the intro
    match orientation {
        Orientation::Vertical => {
            for r in row..(height + row) {
                if col < 0 || r < 0 { continue }
                let _ = s.intro_uni.toggle(col as usize, r as usize, player_id);  // `let _ =`, because we don't care about errors
            }
        }
        Orientation::Horizontal => {
            for c in col..(width + col) {
                if c < 0 || row < 0 { continue }
                let _ = s.intro_uni.toggle(c as usize, row as usize, player_id);
            }
        }
        Orientation::Diagonal => {
            for x in 0..(width - 1) {
                let c: isize = col+x;
                let r: isize = row+x;
                if c < 0 || r < 0 { continue; }
                let _ = s.intro_uni.toggle(c as usize, r as usize, player_id);
            }
        }
    }
}

fn init_title_screen(s: &mut MainState) -> Result<(), ()> {

    // 1) Calculate width and height of rectangle which represents the intro logo
    // 2) Determine height and width of the window
    // 3) Center it
    // 4) get offset for row and column to draw at

    // let resolution = s.video_settings.get_active_resolution();
    let resolution = (DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT);
    let win_width  = resolution.0 as i32 / s.viewport.get_cell_size() as i32;
    let win_height = resolution.1 as i32 / s.viewport.get_cell_size() as i32;
    let player_id = 0;   // hardcoded for this intro

    let letter_width = 5;
    let letter_height = 6;

    // 9 letters; account for width and spacing
    let logo_width = 9*5 + 9*5;
    let logo_height = letter_height as i32;

    let mut offset_col = (win_width/2 - logo_width/2) as isize;
    let offset_row = (win_height/2 - logo_height/2) as isize;

    let toggle = |s_: &mut MainState, col: isize, row: isize| {
        if col >= 0 || row >= 0 {
            let _ = s_.intro_uni.toggle(col as usize, row as usize, player_id); // we don't care if an error is returned
        }
    };

    // C
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col, offset_row+1, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col+1, offset_row+letter_height, letter_width-1,letter_height);

    offset_col += 2*letter_width;

    // O
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col, offset_row+1, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col+1, offset_row+letter_height, letter_width-1,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col+letter_width-1, offset_row+1, letter_width,letter_height-1);

    offset_col += 2*letter_width;

    // N
    toggle_line(s, Orientation::Vertical, offset_col, offset_row, letter_width,letter_height+1);
    toggle_line(s, Orientation::Vertical, offset_col+letter_width, offset_row, letter_width,letter_height+1);
    toggle_line(s, Orientation::Diagonal, offset_col+1, offset_row+1, letter_width,letter_height);

    offset_col += 2*letter_width;

    // W
    toggle_line(s, Orientation::Vertical, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col+letter_width, offset_row, letter_width,letter_height+1);
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row+letter_height, letter_width,letter_height);
    toggle(s, offset_col+letter_width/2, offset_row+letter_height-1);
    toggle(s, offset_col+letter_width/2, offset_row+letter_height-2);
    toggle(s, offset_col+letter_width/2+1, offset_row+letter_height-1);
    toggle(s, offset_col+letter_width/2+1, offset_row+letter_height-2);

    offset_col += 2*letter_width;

    // A
    toggle_line(s, Orientation::Vertical, offset_col, offset_row+1, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col+letter_width, offset_row, letter_width,letter_height+1);
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col+1, offset_row+letter_height/2, letter_width-1,letter_height);

    offset_col += 2*letter_width;

    // Y
    toggle(s, offset_col, offset_row);
    toggle(s, offset_col, offset_row+1);
    toggle(s, offset_col, offset_row+2);
    toggle(s, offset_col+letter_height, offset_row);
    toggle(s, offset_col+letter_height, offset_row+1);
    toggle(s, offset_col+letter_height, offset_row+2);
    toggle_line(s, Orientation::Vertical, offset_col+letter_height/2, offset_row+letter_width/2+2, letter_width,letter_height-3);
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row+letter_height/2, letter_width+2,letter_height-1);

    offset_col += 2*letter_width;

    // S
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row+letter_height, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row+letter_height/2, letter_width,letter_height);
    toggle(s, offset_col, offset_row+1);
    toggle(s, offset_col, offset_row+2);
    toggle(s, offset_col+letter_width-1, offset_row+4);
    toggle(s, offset_col+letter_width-1, offset_row+5);

    offset_col += 2*letter_width;

    // T
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col+letter_width/2, offset_row+1, letter_width,letter_height);

    offset_col += 2*letter_width;

    // E
    toggle_line(s, Orientation::Horizontal, offset_col, offset_row, letter_width,letter_height);
    toggle_line(s, Orientation::Vertical, offset_col, offset_row+1, letter_width,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col+1, offset_row+letter_height, letter_width-1,letter_height);
    toggle_line(s, Orientation::Horizontal, offset_col+1, offset_row+letter_height/2, letter_width-2,letter_height);

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

        let universe_width_in_cells  = 256;
        let universe_height_in_cells = 120;

        let config = config::ConfigFile::new();

        let mut vs = video::VideoSettings::new();
        let _ = vs.gather_display_modes(ctx);

        vs.print_resolutions();

/*
 *  FIXME Disabling video module temporarily as we can now leverage ggez 0.4
 */
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
        if GRID_DRAW_STYLE == DrawStyle::Line {
            // black background - "tetris-like"
            color_settings.cell_colors.insert(CellState::Alive(None), Color::new( 1.0, 1.0, 1.0, 1.0));
        } else {
            // light background
            color_settings.cell_colors.insert(CellState::Alive(None), Color::new( 0.0, 0.0, 0.0, 1.0));
        }
        color_settings.cell_colors.insert(CellState::Alive(Some(0)), Color::new(  1.0,   0.0,   0.0, 1.0));  // 0 is red
        color_settings.cell_colors.insert(CellState::Alive(Some(1)), Color::new(  0.0,   0.0,   1.0, 1.0));  // 1 is blue
        color_settings.cell_colors.insert(CellState::Wall,           Color::new(0.617,  0.55,  0.41, 1.0));
        color_settings.cell_colors.insert(CellState::Fog,            Color::new(0.780, 0.780, 0.780, 1.0));

        let small_font = graphics::Font::new(ctx, "//DejaVuSerif.ttf", 20)?;
        let menu_font  = graphics::Font::new(ctx, "//DejaVuSerif.ttf", 20)?;

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

        let intro_universe = 
        {
            let player = PlayerBuilder::new(Region::new(0, 0, 256, 256));
            BigBang::new()
                .width(256)
                .height(256)
                .fog_radius(100)
                .add_players(vec![player])
                .birth()
        };

        let mut s = MainState {
            small_font:          small_font,
            screen:              Screen::Intro(INTRO_DURATION),
            uni:                 bigbang.unwrap(),
            intro_uni:           intro_universe.unwrap(),
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
        init_title_screen(&mut s).unwrap();

        Ok(s)
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let duration = timer::duration_to_f64(timer::get_delta(ctx)); // seconds

        match self.screen {
            Screen::Intro(mut remaining) => {

                remaining -= duration;
                if remaining > INTRO_DURATION - INTRO_PAUSE_DURATION {
                    self.screen = Screen::Intro(remaining);
                } 
                else {
                    if remaining > 0.0 && remaining <= INTRO_DURATION - INTRO_PAUSE_DURATION {
                        self.intro_uni.next();
                        self.screen = Screen::Intro(remaining);
                    }
                    else {
                        self.screen = Screen::Run; // XXX Menu Screen is disabled for the time being
                    }
                }
            }
            Screen::Menu => {
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
                        let container = self.menu_sys.get_menu_container_mut(); 
                        let mainmenu_md = container.get_metadata();
                        mainmenu_md.adjust_index(y);
                    }
                    self.menu_sys.get_controls().set_menu_key_pressed(true);
                }
                else {
                    /////////////////////////
                    //// Non-Arrow key was pressed
                    //////////////////////////

                    if self.return_key_pressed || self.escape_key_pressed {

                        let mut id = {
                            let container = self.menu_sys.get_menu_container();
                            let index = container.get_menu_item_index();
                            let menu_item_list = container.get_menu_item_list();
                            let menu_item = menu_item_list.get(index).unwrap();
                            menu_item.id
                        };

                        if self.escape_key_pressed {
                            id = menu::MenuItemIdentifier::ReturnToPreviousMenu;
                        }

                        match self.menu_sys.menu_state {
                            menu::MenuState::MainMenu => {
                                if !self.escape_key_pressed {
                                    match id {
                                        menu::MenuItemIdentifier::StartGame => {
                                            self.pause_or_resume_game();
                                        }
                                        menu::MenuItemIdentifier::ExitGame => {
                                            self.screen = Screen::Exit;
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
                                            self.video_settings.toggle_fullscreen(ctx);
                                            self.config.set_fullscreen(self.video_settings.is_fullscreen());
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
            Screen::Run => {
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

                    self.viewport.update(self.arrow_input);
                }
            }
            Screen::Exit => {
               let _ = ctx.quit();
            }
        }

        let _ = self.post_update();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);
        graphics::set_background_color(ctx, (0, 0, 0, 1).into());

        match self.screen {
            Screen::Intro(_) => {
                self.draw_intro(ctx)?;
            }
            Screen::Menu => {
                self.menu_sys.draw_menu(&self.video_settings, ctx, self.first_gen_was_drawn);
            }
            Screen::Run => {
                self.draw_universe(ctx)?;
            }
            Screen::Exit => {}
        }

        graphics::present(ctx);
        timer::yield_now();
        Ok(())
    }

    // Note about coordinates: x and y are "screen coordinates", with the origin at the top left of
    // the screen. x becomes more positive going from left to right, and y becomes more positive
    // going top to bottom.
    fn mouse_button_down_event(&mut self,
                               _ctx: &mut Context,
                               button: MouseButton,
                               x: i32,
                               y: i32
                               ) {
        self.input_manager.add(input::InputAction::MouseClick(button, x, y));
    }

    fn mouse_motion_event(&mut self,
                          _ctx: &mut Context,
                          state: MouseState,
                          x: i32,
                          y: i32,
                          _xrel: i32,
                          _yrel: i32
                          ) {
        match self.screen {
            Screen::Intro(_) => {}
            Screen::Menu | Screen::Run => {
                if state.left() && self.drag_draw != None {
                    self.input_manager.add(input::InputAction::MouseDrag(MouseButton::Left, x, y));
                } else {
                    self.input_manager.add(input::InputAction::MouseMovement(x, y));
                }
            }
            Screen::Exit => { unreachable!() }
        }
    }

    fn mouse_button_up_event(&mut self,
                             _ctx: &mut Context,
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

        match self.screen {
            Screen::Intro(_) => {
                self.screen = Screen::Run; // TODO lets just go to the game for now...
                self.menu_sys.reset();
            }
            Screen::Menu | Screen::Run => {
                // TODO for now just quit the game
                if keycode == Keycode::Escape {
                    self.quit_event(ctx);
                }

                self.input_manager.add(input::InputAction::KeyPress(keycode, repeat));
            }
            Screen::Exit => {}
        }
    }

    fn key_up_event(&mut self,
                    _ctx: &mut Context,
                    keycode: Keycode,
                    _keymod: Mod,
                    _repeat: bool
                    ) {
        //self.input_manager.clear_input_start_time();
        self.input_manager.add(input::InputAction::KeyRelease(keycode));
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        let mut do_not_quit = true;

        match self.screen {
            Screen::Run => {
                self.pause_or_resume_game();
            }
            Screen::Menu => {
                // This is currently handled in the return_key_pressed path as well
                self.escape_key_pressed = true;
            }
            Screen::Exit => {
                do_not_quit = false;
            }
            _ => {}
        }

        do_not_quit
    }

}


struct GameOfLifeDrawParams {
    bg_color: Color,
    fg_color: Color,
    player_id: isize, // Player color >=0, Playerless < 0
    draw_counter: bool,
}

impl MainState {

    fn draw_game_of_life(&self, 
                         ctx: &mut Context,
                         universe: &Universe,
                         draw_params: &GameOfLifeDrawParams
                         ) -> GameResult<()> {

        // grid background
        graphics::set_color(ctx, draw_params.bg_color)?;
        graphics::rectangle(ctx,  GRID_DRAW_STYLE.to_draw_mode(), self.viewport.get_viewport())?;

        // grid foreground (dead cells)
        let full_rect = self.viewport.get_rect_from_origin();

        if let Some(clipped_rect) = utils::Graphics::intersection(full_rect, self.viewport.get_viewport()) {
            graphics::set_color(ctx, draw_params.fg_color)?;
            graphics::rectangle(ctx,  GRID_DRAW_STYLE.to_draw_mode(), clipped_rect)?;
        }

        let image = graphics::Image::solid(ctx, 1u16, graphics::WHITE)?; // 1x1 square
        let mut spritebatch = graphics::spritebatch::SpriteBatch::new(image);

        // grid non-dead cells (walls, players, etc.)
        let visibility = if draw_params.player_id >= 0 {
            Some(draw_params.player_id as usize) //XXX, Player One
        } else {
            Some(0)
        };

        universe.each_non_dead_full(visibility, &mut |col, row, state| {
            let color = if draw_params.player_id >= 0 {
                self.color_settings.get_color(Some(state))
            } else {
                self.color_settings.get_random_color()
            };

            if let Some(rect) = self.viewport.get_screen_area(viewport::Cell::new(col, row)) {
                let p = graphics::DrawParam {
                    dest: Point2::new(rect.x, rect.y),
                    scale: Point2::new(rect.w, rect.h), // scaling a 1x1 Image to correct cell size
                    color: Some(color),
                    ..Default::default()
                };

                spritebatch.add(p);
            }
        });

        graphics::draw_ex(ctx, &spritebatch, graphics::DrawParam{ dest: Point2::new(0.0, 0.0), .. Default::default()} )?;
        spritebatch.clear();

        ////////// draw generation counter
        if draw_params.draw_counter {
            let gen_counter_str = universe.latest_gen().to_string();
            let color = Color::new(1.0, 0.0, 0.0, 1.0);
            utils::Graphics::draw_text(ctx, &self.small_font, color, &gen_counter_str, &Point2::new(0.0, 0.0), None);
        }

        ////////////////////// END
        graphics::set_color(ctx, graphics::BLACK)?; // Clear color residue

        Ok(())
    }

    fn draw_intro(&mut self, ctx: &mut Context) -> GameResult<()>{

        let draw_params = GameOfLifeDrawParams {
            bg_color: graphics::BLACK,
            fg_color: graphics::BLACK,
            player_id: -1,
            draw_counter: true,
        };

        self.draw_game_of_life(ctx, &self.intro_uni, &draw_params)
    }

    fn draw_universe(&mut self, ctx: &mut Context) -> GameResult<()> {

        let draw_params = GameOfLifeDrawParams {
            bg_color: self.color_settings.get_color(None),
            fg_color: self.color_settings.get_color(Some(CellState::Dead)),
            player_id: 1, // Current player, TODO sync with Server's CLIENT ID
            draw_counter: true,
        };

        self.first_gen_was_drawn = true;
        self.draw_game_of_life(ctx, &self.uni, &draw_params)
    }

    fn pause_or_resume_game(&mut self) {
        let cur_menu_state = self.menu_sys.menu_state;
        let cur_stage = self.screen;

        match cur_stage {
            Screen::Menu => {
                if cur_menu_state == menu::MenuState::MainMenu {
                    self.screen = Screen::Run;
                    self.running = true;
                }
            }
            Screen::Run => {
                self.screen = Screen::Menu;
                self.menu_sys.menu_state = menu::MenuState::MainMenu;
                self.running = false;
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
                    input::InputAction::MouseMovement(_x, _y) => {}
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

    fn post_update(&mut self) -> GameResult<()> {
        /*
        match self.input_manager.peek() {
            Some(&input::InputAction::KeyPress(keycode, repeat)) => {
                if repeat {
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
                        _ => self.arrow_input = (0, 0)
                    }
                }
            }
            _ => self.arrow_input = (0,0),
        }
        */

        self.arrow_input = (0, 0);
        self.input_manager.expunge();

        Ok(())
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

    let mut cb = ContextBuilder::new("conwayste", "Aaronm04|Manghi")
        .window_setup(conf::WindowSetup::default()
                      .title(format!("{} {} {}", "💥 conwayste", version!().to_owned(),"💥").as_str())
                      .icon("//conwayste.ico")
                      .resizable(false)
                      .allow_highdpi(true)
                      )
        .window_mode(conf::WindowMode::default()
                     .dimensions(DEFAULT_SCREEN_WIDTH as u32, DEFAULT_SCREEN_HEIGHT as u32)
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
