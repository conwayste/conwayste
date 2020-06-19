/*  Copyright 2017-2020 the Conwayste Developers.
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

// "On Windows, with the default console subsystem, a conhost window will automatically spawn for
// your process, if not already attached. With the windows subsystem, that won't happen."
// -PeterRabbit on Rust Community Discord server
// (the default subsystem is "console", apparently)
// https://doc.rust-lang.org/nightly/reference/runtime.html?highlight=subsystem#the-windows_subsystem-attribute
#![windows_subsystem = "windows"]

extern crate conway;
#[macro_use] extern crate custom_error;
#[macro_use] extern crate downcast_rs;
extern crate env_logger;
extern crate ggez;
#[macro_use] extern crate log;
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
mod network;
mod ui;
mod uilayout;
mod video;
mod viewport;

use chrono::Local;
use log::LevelFilter;

use conway::universe::{BigBang, Universe, CellState, Region, PlayerBuilder};
use conway::grids::{CharGrid, BitGrid};
use conway::rle::Pattern;
use conway::ConwayResult;
use conway::error::ConwayError;
use conway::Rotation;

use netwayste::net::NetwaysteEvent;

use ggez::conf;
use ggez::event::*;
use ggez::{GameError, GameResult, Context, ContextBuilder};
use ggez::graphics::{self, Color, DrawParam, Font};
use ggez::nalgebra::{Point2, Vector2};
use ggez::timer;

use std::env;
use std::error::Error;
use std::io::Write; // For env logger
use std::path;
use std::collections::{BTreeMap};
use std::sync::{Arc, Mutex};

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
    colors::*,
};
use input::{MouseAction, ScrollEvent};
use ui::{
    Chatbox,
    ChatboxPublishHandle,
    GameArea,
    Pane,
    TextField,
    UIError,
    EventType,
    context::{
        EmitEvent,
        Event,
        Handled,
        Handler,
        UIContext,
    }
};
use uilayout::UILayout;


#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Screen {
    Intro,
    Menu,
    Options,
    ServerList,
    InRoom,
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
    Exit,         // We're getting ready to quit the game, WRAP IT UP SON
}

// All game state
struct MainState {
    system_font:         Font,
    screen_stack:        Vec<Screen>,       // Where are we in the game (Intro/Menu Main/Running..)
                                            // If the top is Exit, then the game exits
    uni:                 Universe,          // Things alive and moving here
    intro_uni:           Universe,
    first_gen_was_drawn: bool,              // The purpose of this is to inhibit gen calc until the first draw
    color_settings:      ColorSettings,
    uni_draw_params:     UniDrawParams,
    running:             bool,
    video_settings:      video::VideoSettings,
    config:              config::Config,
    viewport:            viewport::GridView,
    intro_viewport:      viewport::GridView,
    inputs:              input::InputManager,
    net_worker:          Arc<Mutex<Option<network::ConwaysteNetWorker>>>,
    recvd_first_resize:  bool,       // work around an apparent ggez bug where the first resize event is bogus

    // Input state
    single_step:         bool,
    arrow_input:         (isize, isize),
    drag_draw:           Option<CellState>,
    insert_mode:         Option<(BitGrid, usize, usize)>,   // pattern to be drawn on click along with width and height;
                                                            // if Some(...), dragging doesn't draw anything
    current_intro_duration:  f64,

    ui_layout:           UILayout,
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

fn get_text_entered_handler(
    mut chatbox_pub_handle: ChatboxPublishHandle,
    net_worker: Arc<Mutex<Option<network::ConwaysteNetWorker>>>,
) -> Handler {

    Box::new(move |
        _obj: &mut dyn EmitEvent,
        uictx: &mut UIContext,
        evt: &Event,
    | -> Result<Handled, Box<dyn Error>> {
        let username = uictx.config.get().user.name.clone();
        let text = evt.text.as_ref().unwrap(); // unwrap OK because the generator will always set to Some(..)
        if text.is_empty() {
            return Ok(Handled::NotHandled);
        }
        let msg = format!("{}: {}", username, text);

        chatbox_pub_handle.add_message(msg);

        if let Some(ref mut netwayste) = *(net_worker.lock().unwrap()) {
            netwayste.try_send(NetwaysteEvent::ChatMessage(text.clone()));
        }
        Ok(Handled::NotHandled)
    })
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
        let intro_universe_width_in_cells  = 256;
        let intro_universe_height_in_cells = 256;

        let mut config = config::Config::new();
        config.load_or_create_default().map_err(|e| {
            let msg = format!("Error while loading config: {:?}", e);
            GameError::FilesystemError(msg)
        })?;

        let mut vs = video::VideoSettings::new();
        graphics::set_resizable(ctx, true)?;

        // On first-run, use default supported resolution
        let (w, h) = config.get_resolution();
        vs.set_resolution(ctx, video::Resolution{w, h}, true)?;

        let is_fullscreen = config.get().video.fullscreen;
        vs.is_fullscreen = is_fullscreen;
        vs.update_fullscreen(ctx)?;

        let intro_viewport = viewport::GridView::new(
            DEFAULT_ZOOM_LEVEL,
            intro_universe_width_in_cells,
            intro_universe_height_in_cells);

        let viewport = viewport::GridView::new(
            config.get().gameplay.zoom,
            universe_width_in_cells,
            universe_height_in_cells);

        let mut color_settings = ColorSettings {
            cell_colors: BTreeMap::new(),
            background:  *UNIVERSE_BG_COLOR,
        };
        color_settings.cell_colors.insert(CellState::Dead, *CELL_STATE_DEAD_COLOR);
        if GRID_DRAW_STYLE == DrawStyle::Line {
            // black background - for a "tetris-like" effect
            color_settings.cell_colors.insert(CellState::Alive(None), *CELL_STATE_BG_FILL_HOLLOW_COLOR);
        } else {
            // light background - default setting
            color_settings.cell_colors.insert(CellState::Alive(None), *CELL_STATE_BG_FILL_SOLID_COLOR);
        }
        color_settings.cell_colors.insert(CellState::Alive(Some(0)), *CELL_STATE_ALIVE_PLAYER_0_COLOR);  // 0 is red
        color_settings.cell_colors.insert(CellState::Alive(Some(1)), *CELL_STATE_ALIVE_PLAYER_1_COLOR);  // 1 is blue
        color_settings.cell_colors.insert(CellState::Wall, *CELL_STATE_WALL_COLOR);
        color_settings.cell_colors.insert(CellState::Fog, *CELL_STATE_FOG_COLOR);

        // Note: fixed-width fonts are required!
        let font = Font::new(ctx, path::Path::new("/telegrama_render.ttf"))
                    .map_err(|e| GameError::FilesystemError(format!("Could not load or find font. {:?}", e)))?;

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
                .width(intro_universe_width_in_cells)
                .height(intro_universe_height_in_cells)
                .fog_radius(100)
                .add_players(vec![player])
                .birth()
        };

        let mut config = config::Config::new();
        config.load_or_create_default().map_err(|e| {
            let msg = format!("Error while loading config: {:?}", e);
            GameError::ConfigError(msg)
        })?;

        let mut ui_layout = UILayout::new(ctx, &config, font.clone()).unwrap(); // TODO: unwrap not OK!

        // Update universe draw parameters for intro
        let intro_uni_draw_params = UniDrawParams {
            bg_color: graphics::BLACK,
            fg_color: graphics::BLACK,
            player_id: -1,
            draw_counter: true,
        };

        // Add textfield handler
        let net_worker = Arc::new(Mutex::new(None));
        let chatbox_pub_handle = {
            let chatbox_id = ui_layout.chatbox_id.clone();
            let w = ui_layout
                .get_screen_layering(Screen::Run).unwrap()
                .get_widget_mut(&chatbox_id).unwrap();
            let chatbox = w.downcast_ref::<Chatbox>().unwrap(); // unwrap OK because we know this ID is for a Chatbox
            chatbox.new_handle()
        };
        let text_entered_handler = get_text_entered_handler(chatbox_pub_handle, net_worker.clone());
        {
            let textfield_id = ui_layout.chatbox_tf_id.clone();
            let w = ui_layout
                .get_screen_layering(Screen::Run).unwrap()
                .get_widget_mut(&textfield_id).unwrap();
            let tf = w.downcast_mut::<TextField>().unwrap();
            tf.on(EventType::TextEntered, text_entered_handler).unwrap(); // unwrap OK because not in handler
        }

        let mut s = MainState {
            screen_stack:        vec![Screen::Intro],
            system_font:         font.clone(),
            uni:                 bigbang.unwrap(),
            intro_uni:           intro_universe.unwrap(),
            first_gen_was_drawn: false,
            uni_draw_params:     intro_uni_draw_params,
            color_settings:      color_settings,
            running:             false,
            video_settings:      vs,
            config:              config,
            viewport:            viewport,
            intro_viewport:      intro_viewport,
            inputs:              input::InputManager::new(),
            net_worker,
            recvd_first_resize:  false,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            insert_mode:         None,
            current_intro_duration:  0.0,
            ui_layout:           ui_layout,
        };

        init_patterns(&mut s).unwrap();

        init_title_screen(&mut s).unwrap();

        Ok(s)
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let duration = timer::duration_to_f64(timer::delta(ctx)); // seconds

        self.receive_net_updates()?;

        let screen = self.get_current_screen();

        // Handle special case screens
        // NOTE: each match arm except default must return
        match screen {
            Screen::Intro => {
                // Any key should skip the intro
                if self.inputs.key_info.key.is_some() || (self.current_intro_duration > INTRO_DURATION) {
                    self.screen_stack.pop();
                    self.screen_stack.push(Screen::Menu);
                    self.inputs.key_info.key = None;

                    // update universe draw params now that intro is gone
                    self.uni_draw_params = UniDrawParams {
                        bg_color: self.color_settings.get_color(None),
                        fg_color: self.color_settings.get_color(Some(CellState::Dead)),
                        player_id: 1, // Current player, TODO sync with Server's CLIENT ID
                        draw_counter: true,
                    };
                } else {
                    self.current_intro_duration += duration;

                    if self.current_intro_duration >= (INTRO_DURATION - INTRO_PAUSE_DURATION) {
                        self.intro_uni.next();
                    }
                }
                return Ok(());
            }
            Screen::Exit => {
               let _ = ggez::event::quit(ctx);
               return Ok(());
            }
            _ => {} // all others handled below
        }
        let key = self.inputs.key_info.key;
        let keymods = self.inputs.key_info.modifier;
        let is_shift = keymods & KeyMods::SHIFT > KeyMods::default();

        let mouse_point = self.inputs.mouse_info.position;
        let origin_point = self.inputs.mouse_info.down_position;

        let mouse_action = self.inputs.mouse_info.action;

        let left_mouse_click = mouse_action == Some(MouseAction::Click) &&
            self.inputs.mouse_info.mousebutton == MouseButton::Left;

        let mut game_area_has_keyboard_focus = false;
        let game_area_id = self.ui_layout.game_area_id.clone();
        match GameArea::widget_from_screen_and_id(&mut self.ui_layout, screen, &game_area_id) {
            Ok(gamearea) => {
                game_area_has_keyboard_focus = gamearea.has_keyboard_focus;
            }
            Err(e) => {
                if screen == Screen::Run {
                    error!("failed to look up GameArea widget: {:?}", e);
                }
            }
        }

        // ==== Handle widget events ====
        let mut game_area_should_ignore_input = false;
        if let Some(layer) = self.ui_layout.get_screen_layering(screen) {
            let update = Event::new_update();
            layer.emit(&update, ctx, &mut self.config, &mut self.screen_stack).unwrap_or_else(|e| {
                error!("Error from layer.emit on update: {:?}", e);
            });

            // TODO: replace with event
            layer.on_hover(&mouse_point);

            if let Some(action) = mouse_action {
                if action == MouseAction::Drag {
                    // TODO: replace with event
                    //layer.on_drag(&origin_point, &mouse_point);
                }
            }

            if left_mouse_click {
                let click_event = Event::new_click(mouse_point, self.inputs.mouse_info.mousebutton, is_shift);
                layer.emit(&click_event, ctx, &mut self.config, &mut self.screen_stack).unwrap_or_else(|e| {
                    error!("Error from layer.emit on left click: {:?}", e);
                });
            }

            if !game_area_has_keyboard_focus {
                if let Some(key) = key {
                    let key_event = Event::new_key_press(mouse_point, key, is_shift);
                    layer.emit(&key_event, ctx, &mut self.config, &mut self.screen_stack).unwrap_or_else(|e| {
                        error!("Error from layer.emit on key press: {:?}", e);
                    });
                    game_area_should_ignore_input = true;
                }
            }

            let mut text_input = vec![];
            std::mem::swap(&mut self.inputs.text_input, &mut text_input);
            for character in text_input {
                let key_event = Event::new_char_press(mouse_point, character, is_shift);
                layer.emit(&key_event, ctx, &mut self.config, &mut self.screen_stack).unwrap_or_else(|e| {
                    error!("Error from layer.emit on key press (text input): {:?}", e);
                });
            }
        }

        if screen == Screen::Run && game_area_has_keyboard_focus && !game_area_should_ignore_input {
            let result = self.process_running_inputs(ctx);
            handle_error!(result,
                UIError => |e| {
                    error!("Received UI Error from process_running_inputs(). {:?}", e);
                },
                else => |e| {
                    error!("Received unexpected error from process_running_inputs(). {:?}", e);
                }
            ).unwrap(); // OK to call unwrap here because there is an else match arm (all errors handled)

        }


        if screen == Screen::Run {
            if self.single_step {
                self.running = false;
            }

            if self.first_gen_was_drawn && (self.running || self.single_step) {
                self.uni.next();     // next generation
                self.single_step = false;
            }

            if !is_shift {
                // Arrow keys (but not Shift-<Arrow>!) move the player's view of the universe around
                self.viewport.update(self.arrow_input);
            }
        }

        // Handle Escape, only if screen was not changed above
        if key == Some(KeyCode::Escape) && screen == self.get_current_screen() {
            if screen == Screen::Menu {
                self.screen_stack.push(Screen::Run);
            } else {
                self.screen_stack.pop();
            }
        }

        let new_screen = self.get_current_screen();
        self.transition_screen(ctx, screen, new_screen).unwrap_or_else(|e| {
            error!("Failed to transition_screen: {:?}", e);
        });

        // HACK: propagate any video-related config settings from UI handlers to self.video_settings
        // TODO: consider removing self.video_settings
        if self.video_settings.is_fullscreen != self.config.get().video.fullscreen {
            self.video_settings.is_fullscreen = self.config.get().video.fullscreen;
            self.video_settings.update_fullscreen(ctx)?;
        }

        self.post_update()?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 1.0].into());

        let current_screen = self.get_current_screen();

        match current_screen {
            Screen::Intro => {
                self.draw_intro(ctx).unwrap_or_else(|e| {
                    error!("Error from draw_intro: {}", e);
                });
            }
            Screen::Menu => {
                ui::draw_text(ctx, self.system_font.clone(), *MENU_TEXT_COLOR, String::from("Main Menu"), &Point2::new(500.0, 100.0))?;
            }
            Screen::Run => {
                self.draw_universe(ctx).unwrap_or_else(|e| {
                    error!("Error from draw_universe: {}", e);
                });
            }
            Screen::InRoom => {
                ui::draw_text(ctx, self.system_font.clone(), *MENU_TEXT_COLOR, String::from("In Room"), &Point2::new(100.0, 100.0))?;
            }
            Screen::ServerList => {
                ui::draw_text(ctx, self.system_font.clone(), *MENU_TEXT_COLOR, String::from("Server List"), &Point2::new(100.0, 100.0))?;
             },
            Screen::Options => {
                ui::draw_text(ctx, self.system_font.clone(), *MENU_TEXT_COLOR, String::from("Options"), &Point2::new(100.0, 100.0))?;
             },
            Screen::Exit => {}
        }

        if let Some(layering) = self.ui_layout.get_screen_layering(current_screen) {
            layering.draw(ctx).unwrap(); // TODO: unwrap not OK!
        }

        graphics::present(ctx)?;
        timer::yield_now();
        Ok(())
    }

    // Note about coordinates: x and y are "screen coordinates", with the origin at the top left of
    // the screen. x becomes more positive going from left to right, and y becomes more positive
    // going top to bottom.
    // Currently only allow one mouse button event at a time (e.g. left+right click not valid)
    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if self.inputs.mouse_info.mousebutton == MouseButton::Other(0) {
            self.inputs.mouse_info.mousebutton = button;
            self.inputs.mouse_info.down_timestamp = Some(Instant::now());
            self.inputs.mouse_info.action = Some(MouseAction::Held);
            self.inputs.mouse_info.position = Point2::new(x,y);
            self.inputs.mouse_info.down_position = Point2::new(x,y);

            if self.inputs.mouse_info.debug_print {
                debug!("{:?} Down", button);
            }
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.inputs.mouse_info.position = Point2::new(x, y);

        // Check that a valid mouse button was held down (but no motion yet), or that we are already
        // dragging the mouse. If either case is true, update the action to reflect that the mouse
        // is being dragged around
        if self.inputs.mouse_info.mousebutton != MouseButton::Other(0)
            && (self.inputs.mouse_info.action == Some(MouseAction::Held) || self.inputs.mouse_info.action == Some(MouseAction::Drag)) {
            self.inputs.mouse_info.action = Some(MouseAction::Drag);

            if self.inputs.mouse_info.debug_print {
                debug!("Dragging {:?}, Current Position {:?}, Time Held: {:?}",
                    self.inputs.mouse_info.mousebutton,
                    self.inputs.mouse_info.position,
                    self.inputs.mouse_info.down_timestamp.unwrap().elapsed());
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        // Register as a click if the same mouse button that clicked down is what triggered the event
        if self.inputs.mouse_info.mousebutton == button {
            self.inputs.mouse_info.action = Some(MouseAction::Click);
            self.inputs.mouse_info.position = Point2::new(x, y);

            if self.inputs.mouse_info.debug_print {
                debug!("Clicked {:?}, Current Position {:?}, Time Held: {:?}",
                    button,
                    (x, y),
                    self.inputs.mouse_info.down_timestamp.unwrap().elapsed());
            }
        }

        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    /// Vertical scroll:   (y, positive away from and negative toward the user)
    /// Horizontal scroll: (x, positive to the right and negative to the left)
    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) {
        self.inputs.mouse_info.scroll_event = if y > 0.0 {
                Some(ScrollEvent::ScrollUp)
            } else if y < 0.0 {
                Some(ScrollEvent::ScrollDown)
            } else {
                None
            };

        if self.inputs.mouse_info.debug_print {
            debug!("Wheel Event {:?}", self.inputs.mouse_info.scroll_event);
        }
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: KeyCode, keymod: KeyMods, repeat: bool ) {
        let key_as_int32 = keycode as i32;

        // Winit's KeyCode definition has no perceptible ordering so I'm selectively defining what keys we'll accept...
        // for now at least
        if key_as_int32 < (KeyCode::Numlock as i32)
            || (key_as_int32 >= KeyCode::LAlt as i32 && key_as_int32 <= KeyCode::LWin as i32)
            || (key_as_int32 >= KeyCode::RAlt as i32 && key_as_int32 <= KeyCode::RWin as i32)
            || (key_as_int32 == KeyCode::Equals as i32 || key_as_int32 == KeyCode::Subtract as i32
            ||  key_as_int32 == KeyCode::Tab as i32) {

            // NOTE: we need to exclude modifiers we are using below.
            let is_modifier_key = keycode == KeyCode::LShift || keycode == KeyCode::RShift;
            if self.inputs.key_info.key.is_none() && !is_modifier_key {
                self.inputs.key_info.key = Some(keycode);
            }

            if self.inputs.key_info.key == Some(keycode) {
                self.inputs.key_info.repeating = repeat;
            }

            self.inputs.key_info.modifier = keymod;
        }

        if self.inputs.key_info.debug_print {
            debug!("Key_Down K: {:?}, M: {:?}, R: {}", self.inputs.key_info.key, self.inputs.key_info.modifier, self.inputs.key_info.repeating);
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, _keycode: KeyCode, keymod: KeyMods) {
        // TODO: should probably only clear key if keycode matches key_info.key
        self.inputs.key_info.modifier &= !keymod;  // clear whatever modifier key was released
        self.inputs.key_info.key = None;
        self.inputs.key_info.repeating = false;

        if self.inputs.key_info.debug_print {
            debug!("Key_Up K: {:?}, M: {:?}, R: {}", self.inputs.key_info.key, self.inputs.key_info.modifier, self.inputs.key_info.repeating);
        }
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) {
        // Ignore control characters (like Esc or Del)./
        if character.is_control() {
            return;
        }

        self.inputs.text_input.push(character);
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
        if self.uni_draw_params.player_id < 0 {
            self.intro_viewport.set_size(width, height);
            self.center_intro_viewport(width, height);
        }
        graphics::set_screen_coordinates(ctx, new_rect).unwrap();
        self.viewport.set_size(width, height);
        if self.video_settings.is_fullscreen {
            debug!("not saving resolution to config because is_fullscreen is true");
        } else {
            self.config.set_resolution(width, height);
        }
        self.video_settings.set_resolution(ctx, video::Resolution{w: width, h: height}, false).unwrap();
    }

    /// Called when the user requests that the window be closed (ggez gets a
    /// WindowEvent::CloseRequested event from winit)
    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        info!("Got quit event!");
        false
        /*
        let mut quit = false;
        let current_screen = match self.screen_stack.last() {
            Some(screen) => screen,
            None => panic!("Error in quit_event! Screen_stack is empty!"),
        };

        match current_screen {
            Screen::Run => {
                self.screen_stack.pop();
                assert_eq!(self.get_current_screen(), Screen::Menu);
                self.transition_screen(ctx, Screen::Run, Screen::Menu);
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
        */
    }

}


struct UniDrawParams {
    bg_color: Color,
    fg_color: Color,
    player_id: isize, // Player color >=0, Playerless < 0  // TODO: use Option<usize> instead
    draw_counter: bool,
}

impl MainState {

    fn draw_game_of_life(&self, ctx: &mut Context, universe: &Universe) -> Result<(), Box<dyn Error>> {

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
        let mut main_spritebatch = graphics::spritebatch::SpriteBatch::new(image.clone());
        let mut overlay_spritebatch = graphics::spritebatch::SpriteBatch::new(image);

        // grid non-dead cells (walls, players, etc.)
        let visibility = if self.uni_draw_params.player_id >= 0 {
            Some(self.uni_draw_params.player_id as usize)
        } else {
            // used for random coloring in intro
            Some(0)
        };

        // TODO: call each_non_dead with visible region (add method to viewport)
        universe.each_non_dead_full(visibility, &mut |col, row, state| {
            let color = if self.uni_draw_params.player_id >= 0 {
                self.color_settings.get_color(Some(state))
            } else {
                self.color_settings.get_random_color()
            };

            if let Some(rect) = viewport.window_coords_from_game(viewport::Cell::new(col, row)) {
                let p = graphics::DrawParam::new()
                    .dest(Point2::new(rect.x, rect.y))
                    .scale(Vector2::new(rect.w, rect.h))
                    .color(color);

                main_spritebatch.add(p);
            }
        });

        // TODO: truncate if outside of writable region
        // TODO: move to new function
        if let Some((ref grid, width, height)) = self.insert_mode {
            let unwritable_flash_on =  timer::time_since_start(ctx).subsec_millis() % 250 < 125;  // 50% duty cycle, 250ms period

            if self.uni_draw_params.player_id < 0 {
                return Err(format!("Unexpected player ID {}", self.uni_draw_params.player_id).into());
            }
            let player_cell_state = CellState::Alive(Some(self.uni_draw_params.player_id as usize));
            let player_color = self.color_settings.get_color(Some(player_cell_state));
            if let Some(cursor_cell) = viewport.game_coords_from_window(self.inputs.mouse_info.position) {
                let (cursor_col, cursor_row) = (cursor_cell.col, cursor_cell.row);
                grid.each_set(|grid_col, grid_row| {
                    let col = (grid_col + cursor_col) as isize - width as isize/2;
                    let row = (grid_row + cursor_row) as isize - height as isize/2;
                    if col < 0 || row < 0 {
                        // out of range
                        return;
                    }
                    let (col, row) = (col as usize, row as usize);
                    if let Some(rect) = viewport.window_coords_from_game(viewport::Cell::new(col, row)) {
                        let mut color = player_color;
                        // only error is due to player_id out of range, so unwrap OK here
                        if !self.uni.writable(col, row, self.uni_draw_params.player_id as usize).unwrap() {
                            // not writable, so draw flashing red cells
                            if unwritable_flash_on {
                                color = *constants::colors::INSERT_PATTERN_UNWRITABLE;
                            } else {
                                return;
                            }
                        }
                        color.a = 0.5;  // semi-transparent since this is an overlay
                        let p = graphics::DrawParam::new()
                            .dest(Point2::new(rect.x, rect.y))
                            .scale(Vector2::new(rect.w, rect.h))
                            .color(color);

                        overlay_spritebatch.add(p);
                    }
                });
            }
        }

        if let Some(clipped_rect) = ui::intersection(full_rect, viewport_rect) {
            let origin = graphics::DrawParam::new().dest(Point2::new(0.0, 0.0));
            let rectangle = graphics::Mesh::new_rectangle(ctx, GRID_DRAW_STYLE.to_draw_mode(), clipped_rect,
                                                          self.uni_draw_params.fg_color)?;

            graphics::draw(ctx, &rectangle, origin)?;
            graphics::draw(ctx, &main_spritebatch, origin)?;
            graphics::draw(ctx, &overlay_spritebatch, origin)?;
        }

        // TODO: see if we need to do this
        main_spritebatch.clear();
        overlay_spritebatch.clear();

        ////////// draw generation counter
        if self.uni_draw_params.draw_counter {
            let gen_counter = universe.latest_gen().to_string();
            ui::draw_text(ctx, self.system_font.clone(), *GEN_COUNTER_COLOR, gen_counter, &Point2::new(0.0, 0.0))?;
        }

        Ok(())
    }

    fn center_intro_viewport(&mut self, win_width: f32, win_height: f32) {
        let grid_width = self.intro_viewport.grid_width();
        let grid_height = self.intro_viewport.grid_height();
        let target_center_x = win_width/2.0 - grid_width/2.0;
        let target_center_y = win_height/2.0 - grid_height/2.0;
        self.intro_viewport.set_origin(Point2::new(target_center_x, target_center_y));
    }

    fn draw_intro(&mut self, ctx: &mut Context) -> Result<(), Box<dyn Error>> {
        self.draw_game_of_life(ctx, &self.intro_uni)
    }

    fn draw_universe(&mut self, ctx: &mut Context) -> Result<(), Box<dyn Error>> {
        self.first_gen_was_drawn = true;
        self.draw_game_of_life(ctx, &self.uni)
    }

    fn transition_screen(&mut self, ggez_ctx: &mut Context, old_screen: Screen, new_screen: Screen) -> Result<(), Box<dyn Error>> {
        match old_screen {
            Screen::Menu => {
                if new_screen == Screen::Run {
                    let id = self.ui_layout.game_area_id.clone();
                    if let Some(layering) = self.ui_layout.get_screen_layering(Screen::Run) {
                        layering.enter_focus(ggez_ctx, &mut self.config, &mut self.screen_stack, &id)?;
                    }
                    self.running = true;
                }
            }
            Screen::Run => {
                if new_screen == Screen::Menu {
                    self.running = false;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles keyboard and mouse input stored in `self.inputs` by the ggez callbacks. This is
    /// called from update() when we are in the Run screen, and the focus is not captured by, for
    /// example, a text dialog.
    fn process_running_inputs(&mut self, ctx: &mut Context) -> Result<(), Box<dyn Error>> {
        let keymods = self.inputs.key_info.modifier;
        let is_shift = keymods & KeyMods::SHIFT > KeyMods::default();

        if let Some(keycode) = self.inputs.key_info.key {
            match keycode {
                KeyCode::Key1 => {
                    // pressing 1 clears selection
                    self.insert_mode = None;
                }
                k if k >= KeyCode::Key2 && k <= KeyCode::Key0 => {
                    // select a pattern
                    let grid_info_result = self.bit_pattern_from_char(keycode);
                    let grid_info = handle_error!{grid_info_result -> (BitGrid, usize, usize),
                        ConwayError => |e| {
                            return Err(format!("Invalid pattern bound to keycode {:?}: {}", keycode, e).into())
                        }
                    }?;
                    self.insert_mode = Some(grid_info);
                }
                KeyCode::Return => {
                    let chatbox_pane_id = self.ui_layout.chatbox_pane_id.clone();
                    match Pane::widget_from_screen_and_id(&mut self.ui_layout, Screen::Run, &chatbox_pane_id) {
                        Ok(chatbox_pane) => {
                            if let Some(layer) = self.ui_layout.get_screen_layering(Screen::Run) {
                                layer.enter_focus(ctx, &mut self.config, &mut self.screen_stack, &chatbox_pane_id)?;
                            }
                        }
                        Err(e) => {
                            error!("Could not get Chatbox's textfield while processing key inputs: {:?}", e);
                        }
                    }
                }
                KeyCode::R => {
                    if !self.inputs.key_info.repeating {
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

        if self.inputs.mouse_info.mousebutton == MouseButton::Left {
            let mouse_pos = self.inputs.mouse_info.position;

            if let Some((ref grid, width, height)) = self.insert_mode {
                // inserting a pattern
                if self.inputs.mouse_info.action == Some(MouseAction::Click) {
                    if let Some(cell) = self.viewport.get_cell(mouse_pos) {
                        let insert_col = cell.col as isize - (width/2) as isize;
                        let insert_row = cell.row as isize - (height/2) as isize;
                        let dst_region = Region::new(insert_col, insert_row, width, height);
                        self.uni.copy_from_bit_grid(grid, dst_region, Some(CURRENT_PLAYER_ID));
                    }
                }
            } else {
                // not inserting a pattern, just drawing single cells
                match self.inputs.mouse_info.action {
                    Some(MouseAction::Click) => {
                        // release
                        self.drag_draw = None;
                    }
                    Some(MouseAction::Drag) => {
                        // hold + motion
                        if let Some(cell) = self.viewport.get_cell(mouse_pos) {
                            // Only make dead cells alive
                            if let Some(cell_state) = self.drag_draw {
                                self.uni.set(cell.col, cell.row, cell_state, CURRENT_PLAYER_ID);
                            }
                        }
                    }
                    Some(MouseAction::Held) => {
                        // depress, no move yet
                        if let Some(cell) = self.viewport.get_cell(mouse_pos) {
                            if self.drag_draw.is_none() {
                                self.drag_draw = self.uni.toggle(cell.col, cell.row, CURRENT_PLAYER_ID).ok();
                            }
                        }
                    }
                    Some(MouseAction::DoubleClick) | None => {} // do nothing
                }
            }
        } else if is_shift && self.arrow_input != (0, 0) {
            if let Some((ref mut grid, ref mut width, ref mut height)) = self.insert_mode {
                let rotation = match self.arrow_input {
                    (-1, 0) => Some(Rotation::CCW),
                    ( 1, 0) => Some(Rotation::CW),
                    (0, 0) => unreachable!(),
                    _ => None,   // do nothing in this case
                };
                if let Some(rotation) = rotation {
                    grid.rotate(*width, *height, rotation).unwrap_or_else(|e| {
                        error!("Failed to rotate pattern {:?}: {:?}", rotation, e);
                    });
                    // reverse the stored width and height
                    let (new_width, new_height) = (*height, *width);
                    *width = new_width;
                    *height = new_height;
                } else {
                    info!("Ignoring Shift-<Up/Down>");
                }
            }
        }
        Ok(())
    }

    // update
    fn receive_net_updates(&mut self) -> GameResult<()> {
        let mut net_worker_guard = self.net_worker.lock().unwrap();
        if net_worker_guard.is_none() {
            return Ok(());
        }

        let mut incoming_messages = vec![];

        let net_worker = net_worker_guard.as_mut().unwrap();
        for e in net_worker.try_receive().into_iter() {
            match e {
                NetwaysteEvent::LoggedIn(server_version) => {
                    info!("Logged in! Server version: v{}", server_version);
                    self.screen_stack.push(Screen::ServerList); // XXX
                    // do other stuff
                    net_worker.try_send(NetwaysteEvent::List);
                    net_worker.try_send(NetwaysteEvent::JoinRoom("general".to_owned()));
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
                        println!("{:?}", m); // print to stdout for dbg

                        incoming_messages.push(msg);
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

        let id = self.ui_layout.chatbox_id.clone();
        for msg in incoming_messages {
            match Chatbox::widget_from_screen_and_id(&mut self.ui_layout, Screen::Run, &id) {
                Ok(cb) => cb.add_message(msg),
                Err(e) => error!("Could not add message to Chatbox on network message receive: {:?}", e)
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
                    self.inputs.mouse_info.mousebutton = MouseButton::Other(0);
                    self.inputs.mouse_info.down_position = Point2::new(0.0, 0.0);
                }
                MouseAction::Drag | MouseAction::Held | MouseAction::DoubleClick => {}
            }
        }
        self.inputs.mouse_info.scroll_event = None;

        if self.inputs.key_info.key.is_some() {
            self.inputs.key_info.key = None;
        }

        self.arrow_input = (0, 0);

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

    fn get_current_screen(&self) -> Screen {
        match self.screen_stack.last() {
            Some(screen) => *screen,
            None => panic!("Error in main thread draw! Screen_stack is empty!"),
        }
    }

    /// This takes a keyboard code and returns a `Result` whose Ok value is a `(BitGrid, width,
    /// height)` tuple.
    ///
    /// # Errors
    ///
    /// This will return an error if the selected RLE pattern is invalid.
    fn bit_pattern_from_char(&self, keycode: KeyCode) -> Result<(BitGrid, usize, usize), Box<dyn Error>> {
        let gameplay = &self.config.get().gameplay;
        let rle_str = match keycode {
            KeyCode::Key2 => &gameplay.pattern2,
            KeyCode::Key3 => &gameplay.pattern3,
            KeyCode::Key4 => &gameplay.pattern4,
            KeyCode::Key5 => &gameplay.pattern5,
            KeyCode::Key6 => &gameplay.pattern6,
            KeyCode::Key7 => &gameplay.pattern7,
            KeyCode::Key8 => &gameplay.pattern8,
            KeyCode::Key9 => &gameplay.pattern9,
            KeyCode::Key0 => &gameplay.pattern0,
            _ => "", // unexpected
        };
        let pat = Pattern(rle_str.to_owned());
        let (width, height) = pat.calc_size()?;  // calc_size will fail on invalid RLE -- return it
        let grid = pat.to_new_bit_grid(width, height)?;
        Ok((grid, width, height))
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

// TODO: this should really have "intro" in its name!
fn init_title_screen(s: &mut MainState) -> Result<(), ()> {

    // 1) Calculate width and height of rectangle which represents the intro logo
    // 2) Determine height and width of the window
    // 3) Center it
    // 4) get offset for row and column to draw at

    let player_id = 0;   // hardcoded for this intro

    let letter_width = 5;
    let letter_height = 6;

    // 9 letters; account for width and spacing
    let logo_width = 9*5 + 9*5;
    let logo_height = letter_height;

    let uni_width = s.intro_uni.width() as isize;
    let uni_height = s.intro_uni.height() as isize;

    let mut offset_col = uni_width/2  - logo_width/2;
    let     offset_row = uni_height/2 - logo_height/2;

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
                Local::now().format("%H:%M:%S%.6f"),
                record.level(),
                record.args(),
            )
        })
        .filter(None, LevelFilter::Debug)
        .filter(Some("futures"), LevelFilter::Info)
        .filter(Some("tokio_core"), LevelFilter::Info)
        .filter(Some("tokio_reactor"), LevelFilter::Info)
        .filter(Some("conway"), LevelFilter::Info)
        .filter(Some("ggez"), LevelFilter::Warn)
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
        info!("Found CARGO_MANIFEST_DIR; Adding ${{CARGO_MANIFEST_DIR}}/resources path: {:?}", path);
        cb = cb.add_resource_path(path);
    }

    let (ctx, events_loop) = &mut cb.build().unwrap_or_else(|e| {
        error!("ContextBuilder failed: {:?}", e);
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
                game.cleanup();
                println!("Game exited cleanly.");
            }
        }
    }
}
