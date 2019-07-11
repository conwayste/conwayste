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
#[macro_use] extern crate custom_error;
extern crate env_logger;
extern crate ggez;
#[macro_use] extern crate log;
extern crate sdl2;
#[macro_use] extern crate serde;
#[macro_use] extern crate version;
extern crate rand;
extern crate color_backtrace;
#[macro_use] extern crate lazy_static;
extern crate chromatica;

mod config;
mod constants;
mod error;
mod input;
mod menu;
mod network;
mod ui;
mod video;
mod viewport;

use chrono::Local;
use chromatica::css;
use log::LevelFilter;

use conway::universe::{BigBang, Universe, CellState, Region, PlayerBuilder};
use conway::grids::CharGrid;
use conway::rle::Pattern;
use conway::ConwayResult;

use netwayste::net::NetwaysteEvent;

use ggez::conf;
use ggez::event::*;
use ggez::{GameError, GameResult, Context, ContextBuilder};
use ggez::graphics;
use ggez::graphics::{Point2, Color, Rect};
use ggez::timer;

use std::env;
use std::io::Write; // For env logger
use std::path;
use std::collections::BTreeMap;
use std::time::Instant;

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
use input::{MouseAction, ScrollEvent};
use error::{ConwaysteResult, ConwaysteError::*};
use viewport::Cell;
use ui::{
    Widget,
    Button,
    Checkbox, ToggleState,
    Chatbox, //TextState,
    Pane,
    Screen,
    UIAction,
    WidgetID,
};

// All game state
struct MainState {
    small_font:          graphics::Font,
    menu_font:           graphics::Font,
    screen_stack:        Vec<Screen>,       // Where are we in the game (Intro/Menu Main/Running..)
    uni:                 Universe,          // Things alive and moving here
    intro_uni:           Universe,
    first_gen_was_drawn: bool,              // The purpose of this is to inhibit gen calc until the first draw
    color_settings:      ColorSettings,
    running:             bool,
    menu_sys:            menu::MenuSystem,
    video_settings:      video::VideoSettings,
    config:              config::Config,
    viewport:            viewport::Viewport,
    inputs:              input::InputManager,
    net_worker:          Option<network::ConwaysteNetWorker>,

    // Input state
    single_step:         bool,
    arrow_input:         (isize, isize),
    drag_draw:           Option<CellState>,
    toggle_paused_game:  bool,

    // Temp place holder for testing ui widgets
    chatbox:             Chatbox,
    pane:                Pane,
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

fn init_patterns(s: &mut MainState) -> ConwayResult<()> {
    let _pat = Pattern("10$10b16W$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW$10bW$10bW$10b16W48$100b2A5b2A$100b2A5b2A2$104b2A$104b2A5$122b2Ab2A$121bA5bA$121bA6bA2b2A$121b3A3bA3b2A$126bA!".to_owned());
    //XXX apply to universe, then return Ok
    //XXX return Ok(());
    // TODO: remove the following
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

    //Wall in player 0 area!
    let bw = 5; // buffer width

    // right side
    for row in (70-bw)..(83+bw+1) {
        s.uni.set_unchecked(132+bw, row, CellState::Wall);
    }

    // top side
    for col in (100-bw)..109 {
        s.uni.set_unchecked(col, 70-bw, CellState::Wall);
    }
    for col in 114..(132+bw+1) {
        s.uni.set_unchecked(col, 70-bw, CellState::Wall);
    }

    // left side
    for row in (70-bw)..(83+bw+1) {
        s.uni.set_unchecked(100-bw, row, CellState::Wall);
    }

    // bottom side
    for col in (100-bw)..120 {
        s.uni.set_unchecked(col, 83+bw, CellState::Wall);
    }
    for col in 125..(132+bw+1) {
        s.uni.set_unchecked(col, 83+bw, CellState::Wall);
    }

    //Wall in player 1!
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

        let universe_width_in_cells  = 256;
        let universe_height_in_cells = 120;

        let mut config = config::Config::new();
        config.load_or_create_default().map_err(|e| {
            let msg = format!("Error while loading config: {:?}", e);
            GameError::from(msg)
        })?;

        let mut vs = video::VideoSettings::new();
        vs.gather_display_modes(ctx)?;

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

        let viewport = viewport::Viewport::new(config.get().gameplay.zoom, universe_width_in_cells, universe_height_in_cells);

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

        let small_font = graphics::Font::new(ctx, "//DejaVuSerif.ttf", 12)?;
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

        let mut config = config::Config::new();
        config.load_or_create_default().map_err(|e| {
            let msg = format!("Error while loading config: {:?}", e);
            GameError::from(msg)
        })?;

        let mut vs = video::VideoSettings::new();
        vs.gather_display_modes(ctx)?;

        vs.print_resolutions();

        let chatbox = Chatbox::new(WidgetID::InGamePane1Chatbox, 5);

        let checkbox = Box::new(Checkbox::new( &menu_font,
            "Toggle FullScreen",
            Rect::new(10.0, 210.0, 20.0, 20.0),
            WidgetID::MainMenuTestCheckbox,
            UIAction::Toggle( if vs.is_fullscreen { ToggleState::Enabled } else { ToggleState::Disabled } ),
        ));

        // Create a new pane, and add two test buttons to it. Actions do not really matter for now, WIP
        let mut pane = Pane::new(WidgetID::MainMenuPane1, Rect::new_i32(20, 20, 300, 250));
        let mut pane_button = Box::new(Button::new(&small_font, "ServerList", WidgetID::MainMenuPane1ButtonYes, UIAction::ScreenTransition(Screen::ServerList)));
        pane_button.set_size(Rect::new(10.0, 10.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(&small_font, "InRoom", WidgetID::MainMenuPane1ButtonNo, UIAction::ScreenTransition(Screen::InRoom)));
        pane_button.set_size(Rect::new(10.0, 70.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(&small_font, "StartGame", WidgetID::MainMenuTestButton, UIAction::ScreenTransition(Screen::Run)));
        pane_button.set_size(Rect::new(10.0, 130.0, 180.0, 50.0));
        pane.add(pane_button);

        pane.add(checkbox);

        let mut s = MainState {
            small_font:          small_font,
            menu_font:           menu_font.clone(),
            screen_stack:        vec![Screen::Intro(INTRO_DURATION)],
            uni:                 bigbang.unwrap(),
            intro_uni:           intro_universe.unwrap(),
            first_gen_was_drawn: false,
            color_settings:      color_settings,
            running:             false,
            menu_sys:            menu::MenuSystem::new(menu_font),
            video_settings:      vs,
            config:              config,
            viewport:            viewport,
            inputs:              input::InputManager::new(),
            net_worker:          None,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            toggle_paused_game:  false,
            chatbox: chatbox,
            pane: pane,
        };

        init_patterns(&mut s).unwrap();
        init_title_screen(&mut s).unwrap();

        Ok(s)
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let duration = timer::duration_to_f64(timer::get_delta(ctx)); // seconds

        self.receive_net_updates(ctx)?;

        let current_screen = match self.screen_stack.last() {
            Some(screen) => screen,
            None => panic!("Error in main thread update! Screen_stack is empty!"),
        };

        match current_screen {
            Screen::Intro(mut remaining) => {

                self.screen_stack.pop();

                // Any key should skip the intro
                if self.inputs.key_info.key.is_some() {
                    self.screen_stack.push(Screen::Menu);
                } else {
                    remaining -= duration;
                    if remaining > INTRO_DURATION - INTRO_PAUSE_DURATION {
                        self.screen_stack.push(Screen::Intro(remaining));
                    }
                    else {
                        if remaining > 0.0 && remaining <= INTRO_DURATION - INTRO_PAUSE_DURATION {
                            self.intro_uni.next();

                            self.screen_stack.push(Screen::Intro(remaining));
                        }
                        else {
                            self.screen_stack.push(Screen::Menu);
                        }
                    }
                }
            }
            Screen::Menu => {
                self.update_current_screen(ctx);
            }
            Screen::Run => {
                // TODO Disable FSP limit until we decide if we need it
                // while timer::check_update_time(ctx, FPS) {
                {
                    self.process_running_inputs();
                    if self.inputs.mouse_info.mousebutton == MouseButton::Left {
                        let (x,y) = self.inputs.mouse_info.position;
                        let mouse_pos = Point2::new(x as f32, y as f32);

                        fn flip_cell(ms: &mut MainState, cell: Cell) {
                            // Make dead cells alive or alive cells dead
                            let result = ms.uni.toggle(cell.col, cell.row, CURRENT_PLAYER_ID);
                            ms.drag_draw = match result {
                                Ok(state) => Some(state),
                                Err(_)    => None,
                            };
                        }

                        match self.inputs.mouse_info.action {
                            Some(MouseAction::Click) => {
                                if let Some(cell) = self.viewport.get_cell(mouse_pos) {
                                    flip_cell(self, cell)
                                }
                            }
                            Some(MouseAction::Drag) => {
                                if let Some(cell) = self.viewport.get_cell(mouse_pos) {
                                    // Only make dead cells alive
                                    if let Some(cell_state) = self.drag_draw {
                                        self.uni.set(cell.col, cell.row, cell_state, CURRENT_PLAYER_ID);
                                    } else {
                                        flip_cell(self, cell)
                                    }
                                }
                            }
                            Some(MouseAction::Held) | None => {} // do nothing
                        }
                    }

                    let mouse_point = Point2::new(self.inputs.mouse_info.position.0 as f32, self.inputs.mouse_info.position.1 as f32);
                    self.chatbox.on_hover(&mouse_point);

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

                    self.viewport.update(self.arrow_input);
                }

                self.viewport.update(self.arrow_input);
            }
            Screen::InRoom => {
                // TODO implement
                if let Some(_k) = self.inputs.key_info.key {
                    println!("Leaving InRoom to ServerList");
                    self.screen_stack.pop(); // for testing, go back to main menu so we can get to the game
                }
            }
            Screen::ServerList => {
                if let Some(_k) = self.inputs.key_info.key {
                    println!("Leaving ServerList to MainMenu");
                    self.screen_stack.pop(); // for testing, go back to main menu so we can get to the game
                }
                // TODO implement
             },
            Screen::Exit => {
               let _ = ctx.quit();
            }
        }

        self.post_update()?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);
        graphics::set_background_color(ctx, (0, 0, 0, 1).into());

        let current_screen = match self.screen_stack.last() {
            Some(screen) => screen,
            None => panic!("Error in main thread draw! Screen_stack is empty!"),
        };

        match current_screen {
            Screen::Intro(_) => {
                self.draw_intro(ctx)?;
            }
            Screen::Menu => {
                self.menu_sys.draw_menu(&self.video_settings, ctx, self.first_gen_was_drawn);

                self.pane.draw(ctx, &self.menu_font)?;
            }
            Screen::Run => {
                self.draw_universe(ctx)?;
                self.chatbox.draw(ctx, &self.small_font)?;
            }
            Screen::InRoom => {
                ui::draw_text(ctx, &self.menu_font, Color::from(css::WHITE), "In Room", &Point2::new(100.0, 100.0), None)?;
                // TODO
            }
            Screen::ServerList => {
                ui::draw_text(ctx, &self.menu_font, Color::from(css::WHITE), "Server List", &Point2::new(100.0, 100.0), None)?;
                // TODO
             },
            Screen::Exit => {}
        }

        graphics::present(ctx);
        timer::yield_now();
        Ok(())
    }

    // Note about coordinates: x and y are "screen coordinates", with the origin at the top left of
    // the screen. x becomes more positive going from left to right, and y becomes more positive
    // going top to bottom.
    // Currently only allow one mouse button event at a time (e.g. left+right click not valid)
    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: i32, y: i32) {
        if self.inputs.mouse_info.mousebutton == MouseButton::Unknown {
            self.inputs.mouse_info.mousebutton = button;
            self.inputs.mouse_info.down_timestamp = Some(Instant::now());
            self.inputs.mouse_info.action = Some(MouseAction::Held);
            self.inputs.mouse_info.position = (x,y);
            self.inputs.mouse_info.down_position = (x,y);

            if self.inputs.mouse_info.debug_print {
                println!("{:?} Down", button);
            }
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, _state: MouseState, x: i32, y: i32, _xrel: i32, _yrel: i32) {
        self.inputs.mouse_info.position = (x, y);

        if self.inputs.mouse_info.mousebutton != MouseButton::Unknown
            && (self.inputs.mouse_info.action == Some(MouseAction::Held) || self.inputs.mouse_info.action == Some(MouseAction::Drag)) {
            self.inputs.mouse_info.action = Some(MouseAction::Drag);

            if self.inputs.mouse_info.debug_print {
                println!("Dragging {:?}, Current Position {:?}, Time Held: {:?}",
                    self.inputs.mouse_info.mousebutton,
                    self.inputs.mouse_info.position,
                    self.inputs.mouse_info.down_timestamp.unwrap().elapsed());
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, x: i32, y: i32) {
        // Register as a click if we ended near where we started
        if self.inputs.mouse_info.mousebutton == button {
            self.inputs.mouse_info.action = Some(MouseAction::Click);
            self.inputs.mouse_info.position = (x, y);

            if self.inputs.mouse_info.debug_print {
                println!("Clicked {:?}, Current Position {:?}, Time Held: {:?}",
                    button,
                    (x, y),
                    self.inputs.mouse_info.down_timestamp.unwrap().elapsed());
            }
        }

        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    /// Vertical scroll:   (y, positive away from and negative toward the user)
    /// Horizontal scroll: (x, positive to the right and negative to the left)
    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: i32, y: i32) {
        self.inputs.mouse_info.scroll_event = if y > 0 {
                Some(ScrollEvent::ScrollUp)
            } else if y < 0 {
                Some(ScrollEvent::ScrollDown)
            } else {
                None
            };

        if self.inputs.mouse_info.debug_print {
            println!("Wheel Event {:?}", self.inputs.mouse_info.scroll_event);
        }
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: Keycode, keymod: Mod, repeat: bool ) {
        let key_as_int32 = keycode as i32;

        // Support just the basics for now by ignoring everything after the last arrow key in the key code list
        if key_as_int32 < (Keycode::NumLockClear as i32)
            || (key_as_int32 >= Keycode::LCtrl as i32 && key_as_int32 < Keycode::Mode as i32) {
            if self.inputs.key_info.key.is_none() {
                self.inputs.key_info.key = Some(keycode);
            }

            if self.inputs.key_info.key == Some(keycode) {
                self.inputs.key_info.repeating = repeat;
            }

            if self.inputs.key_info.modifier.is_none() {
                match keycode {
                    Keycode::LCtrl | Keycode::LGui | Keycode::LAlt | Keycode::LShift |
                    Keycode::RCtrl | Keycode::RGui | Keycode::RAlt | Keycode::RShift => {
                        self.inputs.key_info.modifier = Some(keymod);
                    }
                    _ => {} // ignore all other non-standard, non-modifier keys
                }
            }
        }

        if self.inputs.key_info.debug_print {
            println!("Key_Down K: {:?}, M: {:?}, R: {}", self.inputs.key_info.key, self.inputs.key_info.modifier, self.inputs.key_info.repeating);
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        if self.inputs.key_info.modifier.is_some() {
            match keycode {
                Keycode::LCtrl | Keycode::LGui | Keycode::LAlt | Keycode::LShift |
                Keycode::RCtrl | Keycode::RGui | Keycode::RAlt | Keycode::RShift => {
                    self.inputs.key_info.modifier = None;
                }
                _ => {}, // ignore the non-modifier keys as they're handled below
            }
        }
        self.inputs.key_info.key = None;
        self.inputs.key_info.repeating = false;

        if self.inputs.key_info.debug_print {
            println!("Key_Up K: {:?}, M: {:?}, R: {}", self.inputs.key_info.key, self.inputs.key_info.modifier, self.inputs.key_info.repeating);
        }
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        println!("Got quit event!");
        let mut quit = false;
        let current_screen = match self.screen_stack.last() {
            Some(screen) => screen,
            None => panic!("Error in quit_event! Screen_stack is empty!"),
        };

        match current_screen {
            Screen::Run => {
                self.pause_or_resume_game();
            }
            Screen::Menu | Screen::InRoom | Screen::ServerList => {
                // This is currently handled in the menu processing state path as well
            }
            Screen::Exit => {
                quit = true;
            }
            _ => {}
        }

        if quit {
            self.cleanup();
        }

        !quit
    }

}


struct GameOfLifeDrawParams {
    bg_color: Color,
    fg_color: Color,
    player_id: isize, // Player color >=0, Playerless < 0  // TODO: use Option<usize> instead
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

        if let Some(clipped_rect) = ui::intersection(full_rect, self.viewport.get_viewport()) {
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
            ui::draw_text(ctx, &self.small_font, color, &gen_counter_str, &Point2::new(0.0, 0.0), None)?;
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
        let current_screen = match self.screen_stack.last() {
            Some(screen) => screen,
            None => panic!("Error in key_down_event! Screen_stack is empty!"),
        };

        match current_screen {
            Screen::Menu => {
                if cur_menu_state == menu::MenuState::MainMenu {
                    // If at 1, then we haven't started the game yet
                    if self.screen_stack.len() == 1 {
                        self.screen_stack.push(Screen::Run);
                    } else {
                        self.screen_stack.pop();
                    }
                    self.running = true;
                }
            }
            Screen::Run => {
                self.screen_stack.push(Screen::Menu);
                self.menu_sys.menu_state = menu::MenuState::MainMenu;
                self.running = false;
            }
            _ => unimplemented!()
        }

        self.toggle_paused_game = false;
    }

    fn process_running_inputs(&mut self) {
        let keycode;

        if let Some(k) = self.inputs.key_info.key {
            keycode = k;
        } else {
            return;
        }

        match keycode {
            Keycode::Return => {
                if !self.inputs.key_info.repeating {
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
                let cell_size = self.viewport.get_cell_size();
                self.config.modify(|settings| {
                    settings.gameplay.zoom = cell_size;
                });
            }
            Keycode::Minus | Keycode::Underscore => {
                self.viewport.adjust_zoom_level(viewport::ZoomDirection::ZoomOut);
                let cell_size = self.viewport.get_cell_size();
                self.config.modify(|settings| {
                    settings.gameplay.zoom = cell_size;
                });
            }
            Keycode::D => {
                // TODO: do something with this debug code
                let visibility = None;  // can also do Some(player_id)
                let pat = self.uni.to_pattern(visibility);
                println!("PATTERN DUMP:\n{}", pat.0);
            }
            Keycode::Escape => {
                self.toggle_paused_game = true;
            }
            _ => {
                println!("Unrecognized keycode {}", keycode);
            }
        }
    }

    fn process_menu_inputs(&mut self) {
        let keycode;

        if let Some(k) = self.inputs.key_info.key {
            keycode = k;
        } else {
            return;
        }

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
            _ => {}
        }
    }

    // update
    fn update_current_screen(&mut self, ctx: &mut Context) {
        self.process_menu_inputs();

        let mouse_point = Point2::new(self.inputs.mouse_info.position.0 as f32, self.inputs.mouse_info.position.1 as f32);
        let origin_point = Point2::new(self.inputs.mouse_info.down_position.0 as f32, self.inputs.mouse_info.down_position.1 as f32);

        self.pane.on_hover(&mouse_point);

        if self.inputs.mouse_info.action == Some(MouseAction::Click) && self.inputs.mouse_info.mousebutton == MouseButton::Left {
            if let Some((ui_id, ui_action)) = self.pane.on_click(&mouse_point) {
                self.handle_ui_action(ctx, ui_id, ui_action);
            }
        }


        if let Some(action) =  self.inputs.mouse_info.action {
            if action == MouseAction::Drag {
                self.pane.on_drag(&origin_point, &mouse_point);
            } else if action == MouseAction::Click {
                self.pane.update(true);
            }
        }

        //// Directional Key / Menu movement
        ////////////////////////////////////////
        if self.arrow_input != (0,0) && self.inputs.key_info.key.is_some() {
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
            if let Some(k) = self.inputs.key_info.key {
                let escape_key_pressed = k == Keycode::Escape && !self.inputs.key_info.repeating;
                let return_key_pressed = k == Keycode::Return && !self.inputs.key_info.repeating;

                if !escape_key_pressed && !return_key_pressed {
                    return;
                }

                let mut id = {
                    let container = self.menu_sys.get_menu_container();
                    let index = container.get_menu_item_index();
                    let menu_item_list = container.get_menu_item_list();
                    let menu_item = menu_item_list.get(index).unwrap();
                    menu_item.id
                };

                if escape_key_pressed {
                    id = menu::MenuItemIdentifier::ReturnToPreviousMenu;
                }

                match self.menu_sys.menu_state {
                    menu::MenuState::MainMenu => {
                        if !escape_key_pressed {
                            match id {
                                menu::MenuItemIdentifier::Connect => {
                                    if self.net_worker.is_some() {
                                        info!("already connected! Reconnecting...");
                                    }
                                    let mut net_worker = network::ConwaysteNetWorker::new();
                                    net_worker.connect(self.config.get().user.name.clone());
                                    info!("Connecting...");
                                    self.net_worker = Some(net_worker);

                                }
                                menu::MenuItemIdentifier::StartGame => {
                                    self.pause_or_resume_game();
                                }
                                menu::MenuItemIdentifier::ExitGame => {
                                    self.screen_stack.push(Screen::Exit);;
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
                                if !escape_key_pressed {
                                    self.menu_sys.menu_state = menu::MenuState::Video;
                                }
                            }
                            menu::MenuItemIdentifier::AudioSettings => {
                                if !escape_key_pressed {
                                    self.menu_sys.menu_state = menu::MenuState::Audio;
                                }
                            }
                            menu::MenuItemIdentifier::GameplaySettings => {
                                if !escape_key_pressed {
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
                                if !escape_key_pressed { }
                            }
                        }
                    }
                    menu::MenuState::Gameplay => {
                        match id {
                            menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                self.menu_sys.menu_state = menu::MenuState::Options;
                            }
                            _ => {
                                if !escape_key_pressed { }
                            }
                        }
                    }
                    menu::MenuState::Video => {
                        match id {
                            menu::MenuItemIdentifier::ReturnToPreviousMenu => {
                                self.menu_sys.menu_state = menu::MenuState::Options;
                            }
                            menu::MenuItemIdentifier::Fullscreen => {
                                if !escape_key_pressed {
                                    self.video_settings.toggle_fullscreen(ctx);
                                    let is_fullscreen = self.video_settings.is_fullscreen;
                                    self.config.modify(|settings| {
                                        settings.video.fullscreen = is_fullscreen;
                                    });
                                }
                            }
                            menu::MenuItemIdentifier::Resolution => {
                                if !escape_key_pressed {
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
    }

    // update
    fn receive_net_updates(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.net_worker.is_none() {
            return Ok(());
        }
        let net_worker = self.net_worker.as_mut().unwrap();
        for e in net_worker.try_receive().into_iter() {
            match e {
                NetwaysteEvent::LoggedIn(server_version) => {
                    info!("Logged in! Server version: v{}", server_version);
                    self.screen_stack.push(Screen::ServerList); // XXX
                    // do other stuff
                    net_worker.try_send(NetwaysteEvent::List);
                    net_worker.try_send(NetwaysteEvent::JoinRoom("room".to_owned()));
                }
                NetwaysteEvent::JoinedRoom(room_name) => {
                    println!("Joined Room: {}", room_name);
                    self.screen_stack.push(Screen::InRoom); // XXX
                }
                NetwaysteEvent::PlayerList(list) => {
                    println!("PlayerList: {:?}",list);
                }
                NetwaysteEvent::RoomList(list) => {
                    println!("RoomList: {:?}",list);
                }
                NetwaysteEvent::UniverseUpdate => {
                    println!("Universe update");
                }
                NetwaysteEvent::ChatMessages(msgs) => {
                    for m in msgs {
                        let msg = format!("{}: {}", m.0, m.1);
                        self.chatbox.add_message(ctx, &self.small_font, &msg)?;
                        println!("{:?}", m);
                    }
                }
                NetwaysteEvent::LeftRoom => {
                    println!("Left Room");
                }
                NetwaysteEvent::BadRequest(error) => {
                    println!("Server responded with Bad Request: {:?}", error);
                }
                NetwaysteEvent::ServerError(error) => {
                    println!("Server encountered an error: {:?}", error);
                }
                _ => {
                    panic!("Development panic: Unexpected NetwaysteEvent during netwayste receive update: {:?}", e);
                }
            }
        }

        Ok(())
    }

    fn post_update(&mut self) -> GameResult<()> {
        if let Some(action) = self.inputs.mouse_info.action {
            match action {
                MouseAction::Click => {
                    self.inputs.mouse_info.down_timestamp = None;
                    self.inputs.mouse_info.action = None;
                    self.inputs.mouse_info.mousebutton = MouseButton::Unknown;
                    self.inputs.mouse_info.down_position = (0, 0);
                }
                MouseAction::Drag | MouseAction::Held => {}
            }
        }
        self.inputs.mouse_info.scroll_event = None;

        if self.inputs.key_info.key.is_some() {
            self.inputs.key_info.key = None;
        }

        self.arrow_input = (0, 0);

        // Flush config
        self.config.flush().map_err(|e| {
            GameError::UnknownError(format!("Error while flushing config: {:?}", e))
        })?;

        Ok(())
    }

    // Clean up before we quit
    fn cleanup(&mut self) {
        if self.config.is_dirty() {
            self.config.force_flush().unwrap_or_else(|e| {
                error!("Failed to flush config on exit: {:?}", e);
            });
        }
    }

    fn handle_ui_action(&mut self, ctx: &mut Context, widget_id: WidgetID, action: UIAction) -> ConwaysteResult<()> {
        match widget_id {
            WidgetID::MainMenuPane1ButtonYes | WidgetID::MainMenuPane1ButtonNo | WidgetID::MainMenuTestButton  => {
                match action {
                    UIAction::ScreenTransition(s) => {
                        self.screen_stack.push(s);
                    }
                    _ => {
                        return Err(InvalidUIAction{reason: format!("Widget: {:?}, Action: {:?}", widget_id, action)});
                    }
                }
            },
            WidgetID::MainMenuTestCheckbox => {
                match action {
                    UIAction::Toggle(t) => {
                        if t == ToggleState::Disabled {
                            self.config.modify(|settings| {
                                settings.video.fullscreen = false;
                            });
                        } else {
                            self.config.modify(|settings| {
                                settings.video.fullscreen = true;
                            });
                        }
                        self.video_settings.toggle_fullscreen(ctx);
                    }
                    _ => {
                        return Err(InvalidUIAction{reason: format!("Widget: {:?}, Action: {:?}", widget_id, action)});
                     }
                }
            },
            WidgetID::MainMenuPane1 => {
                return Err(NoAssociatedUIAction{
                    reason: format!("Widget: {:?} is a Pane element and has no associated action", widget_id)
                });
            },
            WidgetID::InGamePane1Chatbox => {
                // TODO
            },
        }

        Ok(())
    }
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

    let resolution = s.video_settings.get_active_resolution();
    // let resolution = (DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT);
    let win_width  = (resolution.0 / DEFAULT_ZOOM_LEVEL) as isize; // cells
    let win_height = (resolution.1 / DEFAULT_ZOOM_LEVEL) as isize; // cells
    let player_id = 0;   // hardcoded for this intro

    let letter_width = 5;
    let letter_height = 6;

    // 9 letters; account for width and spacing
    let logo_width = 9*5 + 9*5;
    let logo_height = letter_height;

    let mut offset_col = win_width/2  - logo_width/2;
    let     offset_row = win_height/2 - logo_height/2;

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



// Now our main function, which does three things:
//
// * First, create a new `ggez::conf::Conf`
// object which contains configuration info on things such
// as screen resolution and window title,
// * Second, create a `ggez::game::Game` object which will
// do the work of creating our MainState and running our game,
// * then just call `game.run()` which runs the `Game` mainloop.
pub fn main() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{:5}] - {}",
                Local::now().format("%a %Y-%m-%d %H:%M:%S%.6f"),
                record.level(),
                record.args(),
            )
        })
        .filter(None, LevelFilter::Trace)
        .filter(Some("futures"), LevelFilter::Off)
        .filter(Some("tokio_core"), LevelFilter::Off)
        .filter(Some("tokio_reactor"), LevelFilter::Off)
        .filter(Some("conway"), LevelFilter::Off)
        .filter(Some("ggez"), LevelFilter::Off)
        .filter(Some("gfx_device_gl"), LevelFilter::Off)
        .init();

    color_backtrace::install();

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

    let ctx = &mut cb.build().unwrap_or_else(|e| {
        error!("ContextBuilter failed: {:?}", e);
        std::process::exit(1);
    });

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
