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
#[macro_use] extern crate serde;
#[macro_use] extern crate version;
extern crate rand;
extern crate color_backtrace;
#[macro_use] extern crate lazy_static;

mod config;
mod constants;
mod input;
mod menu;
mod network;
mod utils;
mod video;
mod viewport;

use chrono::Local;
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
use ggez::graphics::{Color, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::timer;

use std::env;
use std::io::Write; // For env logger
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
enum Stage {
    Intro(f64),   // seconds
    Menu,
    ServerList,
    InRoom,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
}

// All game state
struct MainState {
    small_font:          graphics::Font,
    stage:               Stage,            // Where are we in the game (Intro/Menu Main/Running..)
    uni:                 Universe,          // Things alive and moving here
    intro_uni:           Universe,
    first_gen_was_drawn: bool,              // The purpose of this is to inhibit gen calc until the first draw
    color_settings:      ColorSettings,
    uni_draw_params:     UniDrawParams,
    running:             bool,
    menu_sys:            menu::MenuSystem,
    video_settings:      video::VideoSettings,
    config:              config::Config,
    viewport:            viewport::GridView,
    intro_viewport:      viewport::GridView,
    input_manager:       input::InputManager,
    net_worker:          Option<network::ConwaysteNetWorker>,
    recvd_first_resize:  bool,       // work around an apparent ggez bug where the first resize event is bogus

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
        let universe_width_in_cells  = 256;
        let universe_height_in_cells = 120;

        let mut config = config::Config::new();
        config.load_or_create_default().map_err(|e| {
            let msg = format!("Error while loading config: {:?}", e);
            GameError::FilesystemError(msg)
        })?;

        let mut vs = video::VideoSettings::new();
        graphics::set_resizable(ctx, true)?;

        /* TODO: delete this once we are sure resizable windows are OK.
            vs.gather_display_modes(ctx)?;
            vs.print_resolutions();
        */

        // On first-run, use default supported resolution
        let (w, h) = config.get_resolution();
        vs.set_active_resolution(ctx, w as u32, h as u32)?;

        let is_fullscreen = config.get().video.fullscreen;
        vs.is_fullscreen = is_fullscreen;
        vs.update_fullscreen(ctx)?;

        let intro_viewport = viewport::GridView::new(
            DEFAULT_ZOOM_LEVEL,
            universe_width_in_cells,
            universe_height_in_cells);

        let viewport = viewport::GridView::new(
            config.get().gameplay.zoom,
            universe_width_in_cells,
            universe_height_in_cells);

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

        let font = graphics::Font::default(); // Provides DejaVuSerif.ttf

        /*
         * Game Universe Initialization
         */
        let bigbang =
        {
            // we're going to have to tear this all out when this becomes a real game
            let player0_writable = Region::new(100, 70, 34, 16);
            let player1_writable = Region::new(0, 0, 80, 80);

            let player0 = PlayerBuilder::new(player0_writable);
            let player1 = PlayerBuilder::new(player1_writable);
            let players = vec![player0, player1];

            // TODO we should probably get these settings from the server's universe
            BigBang::new()
            .width(universe_width_in_cells)
            .height(universe_height_in_cells)
            // TODO (libconway#11) setting to false will crash because the `Known` BitGrid is not set up
            .server_mode(true)
            .history(HISTORY_SIZE)
            .fog_radius(FOG_RADIUS)
            .add_players(players)
            .birth()
        };

        /*
         * Introduction Universe Initialization
         */
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

        // Update universe draw parameters for intro
        let intro_uni_draw_params = UniDrawParams {
            bg_color: graphics::BLACK,
            fg_color: graphics::BLACK,
            player_id: -1,
            draw_counter: true,
        };

        /*
         * Network Initialization
         */

        let mut s = MainState {
            small_font:          font.clone(),
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 bigbang.unwrap(),
            intro_uni:           intro_universe.unwrap(),
            first_gen_was_drawn: false,
            uni_draw_params:     intro_uni_draw_params,
            color_settings:      color_settings,
            running:             false,
            menu_sys:            menu::MenuSystem::new(font),
            video_settings:      vs,
            config:              config,
            viewport:            viewport,
            intro_viewport:      intro_viewport,
            input_manager:       input::InputManager::new(input::InputDeviceType::PRIMARY),
            net_worker:          None,
            recvd_first_resize:  false,
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
        let duration = timer::duration_to_f64(timer::delta(ctx)); // seconds

        self.receive_net_updates();

        match self.stage {
            Stage::Intro(mut remaining) => {

                remaining -= duration;
                if remaining > INTRO_DURATION - INTRO_PAUSE_DURATION {
                    self.stage = Stage::Intro(remaining);
                }
                else {
                    if remaining > 0.0 && remaining <= INTRO_DURATION - INTRO_PAUSE_DURATION {
                        self.intro_uni.next();
                        self.stage = Stage::Intro(remaining);
                    } else {
                        // update universe draw params
                        // keep in sync with other places where we transition to Run
                        self.uni_draw_params = UniDrawParams {
                            bg_color: self.color_settings.get_color(None),
                            fg_color: self.color_settings.get_color(Some(CellState::Dead)),
                            player_id: 1, // Current player, TODO sync with Server's CLIENT ID
                            draw_counter: true,
                        };
                        self.stage = Stage::Run; // XXX Menu Stage is disabled for the time being
                    }
                }
            }
            Stage::Menu => {
                self.update_current_screen(ctx); // TODO rewrite for ui changes
            }
            Stage::Run => {
                // while this works at limiting the FPS, its a bit glitchy for input events... that probably should be
                // rewritten as it was hacked together originally
               // while timer::check_update_time(ctx, FPS)

                self.process_running_inputs();

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
            Stage::InRoom => {
                // TODO implement
            }
            Stage::ServerList => {
                // TODO implement
             },
            Stage::Exit => {
               let _ = ggez::event::quit(ctx);
            }
        }

        self.post_update()?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 1.0].into());

        match self.stage {
            Stage::Intro(_) => {
                self.draw_intro(ctx)?;
            }
            Stage::Menu => {
                self.menu_sys.draw_menu(&self.video_settings, ctx, self.first_gen_was_drawn)?;
            }
            Stage::Run => {
                self.draw_universe(ctx)?;
            }
            Stage::InRoom => {
                // TODO
            }
            Stage::ServerList => {
                // TODO
             },
            Stage::Exit => {}
        }

        graphics::present(ctx)?;
        timer::yield_now();
        Ok(())
    }

    // Note about coordinates: x and y are "screen coordinates", with the origin at the top left of
    // the screen. x becomes more positive going from left to right, and y becomes more positive
    // going top to bottom.
    fn mouse_button_down_event(&mut self,
                               _ctx: &mut Context,
                               button: MouseButton,
                               x: f32,
                               y: f32
                               ) {
        self.input_manager.add(input::InputAction::MouseClick(button, x as i32, y as i32));
    }

    fn mouse_motion_event(&mut self,
                          _ctx: &mut Context,
                          x: f32,
                          y: f32,
                          _dx: f32,
                          _dy: f32
                          ) {
        match self.stage {
            Stage::Intro(_) => {}
            Stage::Menu | Stage::Run => {
                if self.drag_draw != None {
                    self.input_manager.add(input::InputAction::MouseDrag(MouseButton::Left, x as i32, y as i32));
                } else {
                    self.input_manager.add(input::InputAction::MouseMovement(x as i32, y as i32));
                }
            }
            Stage::InRoom => {
                // TODO implement
            }
            Stage::ServerList => {
                // TODO implement
             },
            Stage::Exit => { unreachable!() }
        }
    }

    fn mouse_button_up_event(&mut self,
                             _ctx: &mut Context,
                             _button: MouseButton,
                             _x: f32,
                             _y: f32
                             ) {
        // TODO Later, we'll need to support drag-and-drop patterns as well as drag draw
        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    fn key_down_event(&mut self,
                      ctx: &mut Context,
                      keycode: KeyCode,
                      _keymod: KeyMods,
                      repeat: bool
                      ) {

        match self.stage {
            Stage::Intro(_) => {
                // update universe draw params
                // keep in sync with other places where we transition to Run
                self.uni_draw_params = UniDrawParams {
                    bg_color: self.color_settings.get_color(None),
                    fg_color: self.color_settings.get_color(Some(CellState::Dead)),
                    player_id: 1, // Current player, TODO sync with Server's CLIENT ID
                    draw_counter: true,
                };
                self.stage = Stage::Run; // TODO lets just go to the game for now...
                self.menu_sys.reset();
            }
            Stage::Menu | Stage::Run | Stage::InRoom | Stage::ServerList => {
                // TODO for now just quit the game
                if keycode == KeyCode::Escape {
                    self.quit_event(ctx);
                }

                self.input_manager.add(input::InputAction::KeyPress(keycode, repeat));
            }
            Stage::Exit => {}
        }
    }

    fn key_up_event(&mut self,
                    _ctx: &mut Context,
                    keycode: KeyCode,
                    _keymod: KeyMods) {
        //self.input_manager.clear_input_start_time();
        self.input_manager.add(input::InputAction::KeyRelease(keycode));
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        if !self.recvd_first_resize {
            // Work around apparent ggez bug -- bogus first resize_event
            debug!("IGNORING resize_event: {}, {}", width, height);
            self.recvd_first_resize = true;
            return;
        }
        debug!("resize_event: {}, {}", width, height);
        let new_rect = graphics::Rect::new(
            0.0,
            0.0,
            width,
            height,
        );
        graphics::set_screen_coordinates(ctx, new_rect).unwrap();
        self.viewport.set_dimensions(width as u32, height as u32);
        if self.video_settings.is_fullscreen {
            debug!("not saving resolution to config because is_fullscreen is true");
        } else {
            self.config.set_resolution(width as u32, height as u32);
        }
        self.video_settings.resolution = (width as u32, height as u32);
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        let mut quit = false;

        match self.stage {
            Stage::Run => {
                self.pause_or_resume_game();
            }
            Stage::Menu | Stage::InRoom | Stage::ServerList => {
                // This is currently handled in the return_key_pressed path as well
                self.escape_key_pressed = true;
            }
            Stage::Exit => {
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


struct UniDrawParams {
    bg_color: Color,
    fg_color: Color,
    player_id: isize, // Player color >=0, Playerless < 0  // TODO: use Option<usize> instead
    draw_counter: bool,
}

impl MainState {

    fn draw_game_of_life(&self, ctx: &mut Context, universe: &Universe) -> GameResult<()> {

        let viewport = if self.uni_draw_params.player_id >= 0 {
            &self.viewport
        } else {
            // intro
            &self.intro_viewport
        };
        let viewport_rect = viewport.get_rect();

        // grid background
        let rectangle = graphics::Mesh::new_rectangle(ctx,
            GRID_DRAW_STYLE.to_draw_mode(),
            graphics::Rect::new(0.0, 0.0, viewport_rect.w, viewport_rect.h),
            self.uni_draw_params.bg_color)?;
        graphics::draw(ctx, &rectangle, DrawParam::new().dest(viewport_rect.point()))?;

        // grid foreground (dead cells)
        let full_rect = viewport.get_rect_from_origin();

        let image = graphics::Image::solid(ctx, 1u16, graphics::WHITE)?; // 1x1 square
        let mut spritebatch = graphics::spritebatch::SpriteBatch::new(image);

        // grid non-dead cells (walls, players, etc.)
        let visibility = if self.uni_draw_params.player_id >= 0 {
            Some(self.uni_draw_params.player_id as usize)
        } else {
            // used for random coloring in intro
            Some(0)
        };

        universe.each_non_dead_full(visibility, &mut |col, row, state| {
            let color = if self.uni_draw_params.player_id >= 0 {
                self.color_settings.get_color(Some(state))
            } else {
                self.color_settings.get_random_color()
            };

            if let Some(rect) = viewport.get_screen_area(viewport::Cell::new(col, row)) {
                let p = graphics::DrawParam::new()
                    .dest(Point2::new(rect.x, rect.y))
                    .scale(Vector2::new(rect.w, rect.h))
                    .color(color);

                spritebatch.add(p);
            }
        });

        if let Some(clipped_rect) = utils::Graphics::intersection(full_rect, viewport_rect) {
            let origin = graphics::DrawParam::new().dest(Point2::new(0.0, 0.0));
            let rectangle = graphics::Mesh::new_rectangle(ctx, GRID_DRAW_STYLE.to_draw_mode(), clipped_rect,
                                                          self.uni_draw_params.fg_color)?;

            graphics::draw(ctx, &rectangle, origin)?;
            graphics::draw(ctx, &spritebatch, origin)?;
        }

        spritebatch.clear();

        ////////// draw generation counter
        if self.uni_draw_params.draw_counter {
            let gen_counter_str = universe.latest_gen().to_string();
            let color = Color::new(1.0, 0.0, 0.0, 1.0);
            utils::Graphics::draw_text(ctx, &self.small_font, color, &gen_counter_str, &Point2::new(0.0, 0.0), None)?;
        }

        Ok(())
    }

    fn draw_intro(&mut self, ctx: &mut Context) -> GameResult<()>{
        self.draw_game_of_life(ctx, &self.intro_uni)
    }

    fn draw_universe(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.first_gen_was_drawn = true;
        self.draw_game_of_life(ctx, &self.uni)
    }

    fn pause_or_resume_game(&mut self) {
        let cur_menu_state = self.menu_sys.menu_state;
        let cur_stage = self.stage;

        match cur_stage {
            Stage::Menu => {
                if cur_menu_state == menu::MenuState::MainMenu {
                    self.stage = Stage::Run;
                    self.running = true;
                }
            }
            Stage::Run => {
                self.stage = Stage::Menu;
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
                            KeyCode::Return => {
                                if !repeat {
                                    self.running = !self.running;
                                }
                            }
                            KeyCode::Space => {
                                self.single_step = true;
                            }
                            KeyCode::Up => {
                                self.arrow_input = (0, -1);
                            }
                            KeyCode::Down => {
                                self.arrow_input = (0, 1);
                            }
                            KeyCode::Left => {
                                self.arrow_input = (-1, 0);
                            }
                            KeyCode::Right => {
                                self.arrow_input = (1, 0);
                            }
                            KeyCode::Add | KeyCode::Equals => {
                                self.viewport.adjust_zoom_level(viewport::ZoomDirection::ZoomIn);
                                let cell_size = self.viewport.get_cell_size();
                                self.config.modify(|settings| {
                                    settings.gameplay.zoom = cell_size;
                                });
                            }
                            KeyCode::Minus | KeyCode::Subtract => {
                                self.viewport.adjust_zoom_level(viewport::ZoomDirection::ZoomOut);
                                let cell_size = self.viewport.get_cell_size();
                                self.config.modify(|settings| {
                                    settings.gameplay.zoom = cell_size;
                                });
                            }
                            KeyCode::Numpad1 => {
                                self.win_resize = 1;
                            }
                            KeyCode::Numpad2 => {
                                self.win_resize = 2;
                            }
                            KeyCode::Numpad3 => {
                                self.win_resize = 3;
                            }
                            KeyCode::LWin => {

                            }
                            KeyCode::D => {
                                // TODO: do something with this debug code
                                let visibility = None;  // can also do Some(player_id)
                                let pat = self.uni.to_pattern(visibility);
                                println!("PATTERN DUMP:\n{}", pat.0);
                            }
                            _ => {
                                println!("Unrecognized keycode {:?}", keycode);
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
                    /*input::InputAction::MouseRelease(_) => {}*/

                    input::InputAction::KeyPress(keycode, repeat) => {
                        if !self.menu_sys.get_controls().is_menu_key_pressed() {
                            match keycode {
                                KeyCode::Up => {
                                    self.arrow_input = (0, -1);
                                }
                                KeyCode::Down => {
                                    self.arrow_input = (0, 1);
                                }
                                KeyCode::Left => {
                                    self.arrow_input = (-1, 0);
                                }
                                KeyCode::Right => {
                                    self.arrow_input = (1, 0);
                                }
                                KeyCode::Return => {
                                    if !repeat {
                                        self.return_key_pressed = true;
                                    }
                                }
                                KeyCode::Escape => {
                                    self.escape_key_pressed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    input::InputAction::KeyRelease(keycode) => {
                        match keycode {
                            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
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

    // update
    fn update_current_screen(&mut self, ctx: &mut Context) {
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
                                menu::MenuItemIdentifier::Connect => {
                                    if self.net_worker.is_some() {
                                        info!("already connected! Reconnecting...");
                                    }
                                    let mut net_worker = network::ConwaysteNetWorker::new();
                                    net_worker.connect(self.config.get().user.name.clone());
                                    info!("Connected.");
                                    self.net_worker = Some(net_worker);

                                }
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
                                    // toggle
                                    let is_fullscreen = !self.video_settings.is_fullscreen;
                                    self.video_settings.is_fullscreen = is_fullscreen;
                                    // actually update screen based on what we toggled
                                    self.video_settings.update_fullscreen(ctx).unwrap(); // TODO: rollback if fail
                                    // save to persistent config storage
                                    self.config.modify(|settings| {
                                        settings.video.fullscreen = is_fullscreen;
                                    });
                                }
                            }
                            menu::MenuItemIdentifier::Resolution => {
                                // NO-OP; menu item is effectively read-only
                                /*
                                if !self.escape_key_pressed {
                                    self.video_settings.advance_to_next_resolution(ctx);

                                    // Update the configuration file and resize the viewing
                                    // screen
                                    let (w,h) = self.video_settings.get_active_resolution();
                                    self.config.set_resolution(w, h);
                                    self.viewport.set_dimensions(w, h);
                                }
                                */
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

    // update
    fn receive_net_updates(&mut self) {
        if self.net_worker.is_none() {
            return;
        }
        let net_worker = self.net_worker.as_mut().unwrap();
        for e in net_worker.try_receive().into_iter() {
            match e {
                NetwaysteEvent::LoggedIn(server_version) => {
                    info!("Logged in! Server version: v{}", server_version);
                    self.stage = Stage::ServerList; //XXX
                    // do other stuff
                    net_worker.try_send(NetwaysteEvent::List);
                }
                NetwaysteEvent::JoinedRoom(room_name) => {
                    println!("Joined Room: {}", room_name);
                    self.stage = Stage::InRoom;
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
    }

    fn post_update(&mut self) -> GameResult<()> {
        /*
        match self.input_manager.peek() {
            Some(&input::InputAction::KeyPress(keycode, repeat)) => {
                if repeat {
                    match keycode {
                        KeyCode::Up => {
                            self.arrow_input = (0, -1);
                        }
                        KeyCode::Down => {
                            self.arrow_input = (0, 1);
                        }
                        KeyCode::Left => {
                            self.arrow_input = (-1, 0);
                        }
                        KeyCode::Right => {
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

        // Flush config
        self.config.flush().map_err(|e| {
            GameError::FilesystemError(format!("Error while flushing config: {:?}", e))
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
    let win_width  = (resolution.0 as f32 / DEFAULT_ZOOM_LEVEL) as isize; // cells
    let win_height = (resolution.1 as f32 / DEFAULT_ZOOM_LEVEL) as isize; // cells
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
                      .vsync(true)
                      )
        .window_mode(conf::WindowMode::default()
                      .dimensions(DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT)
                      .resizable(false)
                     );

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {:?}", path);
        cb = cb.add_resource_path(path);
    } else {
        println!("Not building from cargo? Okie dokie.");
    }

    let (ctx, events_loop) = &mut cb.build().unwrap_or_else(|e| {
        error!("ContextBuilter failed: {:?}", e);
        std::process::exit(1);
    });

    match MainState::new(ctx) {
        Err(e) => {
            println!("Could not load Conwayste!");
            println!("Error: {}", e);
        }
        Ok(ref mut game) => {
            let result = run(ctx, events_loop, game);
            if let Err(e) = result {
                println!("Error encountered while running game: {}", e);
            } else {
                println!("Game exited cleanly.");
            }
        }
    }
}
