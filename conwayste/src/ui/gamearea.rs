/*  Copyright 2020 the Conwayste Developers.
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

use super::{
    context::{EmitEvent, Event, EventType, Handled, HandlerData, KeyCodeOrChar, UIContext},
    widget::Widget,
    UIError, UIResult,
};
use crate::{config::Config, constants::*, viewport::ZoomDirection};
use conway::{
    error::ConwayError,
    grids::{BitGrid, CharGrid, Rotation},
    rle::Pattern,
    universe::{BigBang, CellState, PlayerBuilder, Region, Universe},
    ConwayResult,
};
use ggez::graphics::Rect;
use ggez::input::keyboard::KeyCode;
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};
use id_tree::NodeId;
use std::error::Error;
use std::fmt;

pub struct GameArea {
    id:                     Option<NodeId>,
    pub has_keyboard_focus: bool,
    z_index:                usize,
    dimensions:             Rect,
    handler_data:           HandlerData,
    pub uni:                Universe,
    game_state:             GameAreaState,
}

impl fmt::Debug for GameArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameArea")
            .field("id", &self.id)
            .field("has_keyboard_focus", &self.has_keyboard_focus)
            .finish()
    }
}

/// For now, this is a dummy widget to represent the actual game area. It may not always be a dummy
/// widget.
impl GameArea {
    pub fn new() -> Self {
        let bigbang = {
            // we're going to have to tear this all out when this becomes a real game
            let player0_writable = Region::new(100, 70, 34, 16);
            let player1_writable = Region::new(0, 0, 80, 80);

            let player0 = PlayerBuilder::new(player0_writable);
            let player1 = PlayerBuilder::new(player1_writable);
            let players = vec![player0, player1];

            BigBang::new()
                .width(UNIVERSE_WIDTH_IN_CELLS)
                .height(UNIVERSE_HEIGHT_IN_CELLS)
                .server_mode(true) // TODO will change to false once we get server support up
                // Currently 'client' is technically both client and server
                .history(HISTORY_SIZE)
                .fog_radius(FOG_RADIUS)
                .add_players(players)
                .birth()
        };
        let mut uni = bigbang.unwrap();

        init_patterns(&mut uni).unwrap();

        let mut game_area = GameArea {
            id:                 None,
            has_keyboard_focus: false,
            z_index:            0,
            dimensions:         Rect::default(),
            handler_data:       HandlerData::new(),
            uni:                uni,
            game_state:         GameAreaState::default(),
        };

        // Set handlers for toggling has_keyboard_focus
        let gain_focus_handler =
            move |obj: &mut dyn EmitEvent, _uictx: &mut UIContext, evt: &Event| -> Result<Handled, Box<dyn Error>> {
                let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
                println!("Game area: Gained Focus");
                game_area.has_keyboard_focus = true;

                Ok(Handled::NotHandled)
            };

        let lose_focus_handler =
            move |obj: &mut dyn EmitEvent, _uictx: &mut UIContext, _evt: &Event| -> Result<Handled, Box<dyn Error>> {
                let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
                println!("Game area: Lost Focus");
                game_area.has_keyboard_focus = false;
                Ok(Handled::NotHandled)
            };

        game_area
            .on(EventType::GainFocus, Box::new(gain_focus_handler))
            .unwrap(); // unwrap OK
        game_area
            .on(EventType::LoseFocus, Box::new(lose_focus_handler))
            .unwrap(); // unwrap OK

        game_area.on(EventType::Update, Box::new(update_handler)).unwrap(); // unwrap OK

        game_area.on(EventType::KeyPress, Box::new(keypress_handler)).unwrap();
        game_area.on(EventType::Click, Box::new(mouse_handler)).unwrap();

        game_area
    }
}

fn init_patterns(uni: &mut Universe) -> ConwayResult<()> {
    let _pat = Pattern("10$10b16W$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW14bW$10bW$10bW$10bW$10b16W48$100b2A5b2A$100b2A5b2A2$104b2A$104b2A5$122b2Ab2A$121bA5bA$121bA6bA2b2A$121b3A3bA3b2A$126bA!".to_owned());

    // Simkin glider gun
    uni.toggle(100, 70, 0)?;
    uni.toggle(100, 71, 0)?;
    uni.toggle(101, 70, 0)?;
    uni.toggle(101, 71, 0)?;

    uni.toggle(104, 73, 0)?;
    uni.toggle(104, 74, 0)?;
    uni.toggle(105, 73, 0)?;
    uni.toggle(105, 74, 0)?;

    uni.toggle(107, 70, 0)?;
    uni.toggle(107, 71, 0)?;
    uni.toggle(108, 70, 0)?;
    uni.toggle(108, 71, 0)?;

    /* eater
    uni.toggle(120, 87, 0)?;
    uni.toggle(120, 88, 0)?;
    uni.toggle(121, 87, 0)?;
    uni.toggle(121, 89, 0)?;
    uni.toggle(122, 89, 0)?;
    uni.toggle(123, 89, 0)?;
    uni.toggle(123, 90, 0)?;
    */

    uni.toggle(121, 80, 0)?;
    uni.toggle(121, 81, 0)?;
    uni.toggle(121, 82, 0)?;
    uni.toggle(122, 79, 0)?;
    uni.toggle(122, 82, 0)?;
    uni.toggle(123, 79, 0)?;
    uni.toggle(123, 82, 0)?;
    uni.toggle(125, 79, 0)?;
    uni.toggle(126, 79, 0)?;
    uni.toggle(126, 83, 0)?;
    uni.toggle(127, 80, 0)?;
    uni.toggle(127, 82, 0)?;
    uni.toggle(128, 81, 0)?;

    uni.toggle(131, 81, 0)?;
    uni.toggle(131, 82, 0)?;
    uni.toggle(132, 81, 0)?;
    uni.toggle(132, 82, 0)?;

    //Wall in player 0 area!
    let bw = 5; // buffer width

    // right side
    for row in (70 - bw)..(83 + bw + 1) {
        uni.set_unchecked(132 + bw, row, CellState::Wall);
    }

    // top side
    for col in (100 - bw)..109 {
        uni.set_unchecked(col, 70 - bw, CellState::Wall);
    }
    for col in 114..(132 + bw + 1) {
        uni.set_unchecked(col, 70 - bw, CellState::Wall);
    }

    // left side
    for row in (70 - bw)..(83 + bw + 1) {
        uni.set_unchecked(100 - bw, row, CellState::Wall);
    }

    // bottom side
    for col in (100 - bw)..120 {
        uni.set_unchecked(col, 83 + bw, CellState::Wall);
    }
    for col in 125..(132 + bw + 1) {
        uni.set_unchecked(col, 83 + bw, CellState::Wall);
    }

    //Wall in player 1!
    for row in 10..19 {
        uni.set_unchecked(25, row, CellState::Wall);
    }
    for col in 10..25 {
        uni.set_unchecked(col, 10, CellState::Wall);
    }
    for row in 11..23 {
        uni.set_unchecked(10, row, CellState::Wall);
    }
    for col in 11..26 {
        uni.set_unchecked(col, 22, CellState::Wall);
    }

    Ok(())
}

fn update_handler(obj: &mut dyn EmitEvent, _uictx: &mut UIContext, _evt: &Event) -> Result<Handled, Box<dyn Error>> {
    let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
    let game_state = &mut game_area.game_state;

    if game_state.first_gen_was_drawn && (game_state.running || game_state.single_step) {
        game_area.uni.next(); // next generation
        game_state.single_step = false;
    }

    Ok(NotHandled)
}

fn keypress_handler(obj: &mut dyn EmitEvent, uictx: &mut UIContext, evt: &Event) -> Result<Handled, Box<dyn Error>> {
    let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK

    if !game_area.has_keyboard_focus {
        return Ok(NotHandled);
    }

    let game_area_state = &mut game_area.game_state;

    if let Some(KeyCodeOrChar::KeyCode(keycode)) = evt.key {
        match keycode {
            KeyCode::Key1 => {
                // pressing 1 clears selection
                game_area_state.insert_mode = None;
            }
            k if k >= KeyCode::Key2 && k <= KeyCode::Key0 => {
                // select a pattern
                let grid_info_result = bit_pattern_from_char(&mut uictx.config, keycode);
                let grid_info = handle_error! {grid_info_result -> (BitGrid, usize, usize),
                    ConwayError => |e| {
                        return Err(format!("Invalid pattern bound to keycode {:?}: {}", keycode, e).into())
                    }
                }?;
                game_area_state.insert_mode = Some(grid_info);
            }
            KeyCode::Return => {
                let chatbox_pane_id = uictx.static_node_ids.chatbox_pane_id.clone();
                uictx.child_event(Event::new_request_focus(chatbox_pane_id));
                /*
                match Pane::widget_from_screen_and_id_mut(&mut self.ui_layout, Screen::Run, &chatbox_pane_id) {
                    Ok(_chatbox_pane) => {
                        if let Some(layer) = self.ui_layout.get_screen_layering_mut(Screen::Run) {
                            layer.enter_focus(
                                ctx,
                                &mut self.config,
                                &mut self.screen_stack,
                                game_area_state.running,
                                &chatbox_pane_id,
                            )?;
                        }
                    }
                    Err(e) => {
                        error!("Could not get Chatbox's textfield while processing key inputs: {:?}", e);
                    }
                }
                */
            }
            KeyCode::R => {
                if !evt.key_repeating {
                    game_area_state.running = !game_area_state.running;
                }
            }
            KeyCode::Space => {
                game_area_state.single_step = true;
            }
            KeyCode::Up => {
                game_area_state.arrow_input = (0, -1);
            }
            KeyCode::Down => {
                game_area_state.arrow_input = (0, 1);
            }
            KeyCode::Left => {
                game_area_state.arrow_input = (-1, 0);
            }
            KeyCode::Right => {
                game_area_state.arrow_input = (1, 0);
            }
            KeyCode::Add | KeyCode::Equals => {
                uictx.viewport.adjust_zoom_level(ZoomDirection::ZoomIn);
                let cell_size = uictx.viewport.get_cell_size();
                uictx.config.modify(|settings| {
                    settings.gameplay.zoom = cell_size;
                });
            }
            KeyCode::Minus | KeyCode::Subtract => {
                uictx.viewport.adjust_zoom_level(ZoomDirection::ZoomOut);
                let cell_size = uictx.viewport.get_cell_size();
                uictx.config.modify(|settings| {
                    settings.gameplay.zoom = cell_size;
                });
            }
            KeyCode::D => {
                // TODO: do something with this debug code
                let visibility = None; // can also do Some(player_id)
                let pat = game_area.uni.to_pattern(visibility);
                println!("PATTERN DUMP:\n{}", pat.0);
            }
            _ => {
                println!("Unrecognized keycode {:?}", keycode);
            }
        }
    }

    Ok(Handled)
}

fn mouse_handler(obj: &mut dyn EmitEvent, uictx: &mut UIContext, evt: &Event) -> Result<Handled, Box<dyn Error>> {
    let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
    let game_area_state = &mut game_area.game_state;
    use ggez::input::mouse::MouseButton;

    if let Some(MouseButton::Left) = evt.button {
        let mouse_pos = evt.point.unwrap(); //unwrap safe b/c mouse clicks must have a point

        if let Some((ref grid, width, height)) = game_area_state.insert_mode {
            // inserting a pattern
            if evt.what == EventType::Click {
                if let Some(cell) = uictx.viewport.get_cell(mouse_pos) {
                    let insert_col = cell.col as isize - (width / 2) as isize;
                    let insert_row = cell.row as isize - (height / 2) as isize;
                    let dst_region = Region::new(insert_col, insert_row, width, height);
                    game_area
                        .uni
                        .copy_from_bit_grid(grid, dst_region, Some(CURRENT_PLAYER_ID));
                }
            }
        } else {
            // not inserting a pattern, just drawing single cells
            match evt.what {
                EventType::Click => {
                    // release
                    game_area_state.drag_draw = None;
                }
                EventType::Drag => {
                    // hold + motion
                    if let Some(cell) = uictx.viewport.get_cell(mouse_pos) {
                        // Only make dead cells alive
                        if let Some(cell_state) = game_area_state.drag_draw {
                            game_area.uni.set(cell.col, cell.row, cell_state, CURRENT_PLAYER_ID);
                        }
                    }
                }
                EventType::MousePressAndHeld => {
                    // depress, no move yet
                    if let Some(cell) = uictx.viewport.get_cell(mouse_pos) {
                        if game_area_state.drag_draw.is_none() {
                            game_area_state.drag_draw =
                                game_area.uni.toggle(cell.col, cell.row, CURRENT_PLAYER_ID).ok();
                        }
                    }
                }
                _ => {}
            }
        }
    } else if evt.shift_pressed && game_area_state.arrow_input != (0, 0) {
        if let Some((ref mut grid, ref mut width, ref mut height)) = game_area_state.insert_mode {
            let rotation = match game_area_state.arrow_input {
                (-1, 0) => Some(Rotation::CCW),
                (1, 0) => Some(Rotation::CW),
                (0, 0) => unreachable!(),
                _ => None, // do nothing in this case
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

    Ok(NotHandled)
}

/// This takes a keyboard code and returns a `Result` whose Ok value is a `(BitGrid, width,
/// height)` tuple.
///
/// # Errors
///
/// This will return an error if the selected RLE pattern is invalid.
fn bit_pattern_from_char(config: &mut Config, keycode: KeyCode) -> Result<(BitGrid, usize, usize), Box<dyn Error>> {
    let gameplay = &config.get().gameplay;
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
    let (width, height) = pat.calc_size()?; // calc_size will fail on invalid RLE -- return it
    let grid = pat.to_new_bit_grid(width, height)?;
    Ok((grid, width, height))
}

impl Widget for GameArea {
    fn id(&self) -> Option<&NodeId> {
        self.id.as_ref()
    }

    fn set_id(&mut self, new_id: NodeId) {
        self.id = Some(new_id);
    }

    fn z_index(&self) -> usize {
        self.z_index
    }

    fn set_z_index(&mut self, new_z_index: usize) {
        self.z_index = new_z_index;
    }

    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!(
                    "Cannot set the size to a width or height of GameArea {:?} to zero",
                    self.id()
                ),
            }));
        }

        self.dimensions = new_dims;
        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.dimensions.point().into()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.dimensions.x = x;
        self.dimensions.y = y;
    }

    fn size(&self) -> (f32, f32) {
        (self.dimensions.w, self.dimensions.h)
    }

    fn set_size(&mut self, w: f32, h: f32) -> UIResult<()> {
        if w == 0.0 || h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of GameArea {:?} to zero", self.id()),
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn draw(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // no-op; dummy widget
        Ok(())
    }

    /// convert to EmitEvent
    fn as_emit_event(&mut self) -> Option<&mut dyn EmitEvent> {
        Some(self)
    }

    /// Whether this widget accepts keyboard events
    fn accepts_keyboard_events(&self) -> bool {
        true
    }
}

impl_emit_event!(GameArea, self.handler_data);
widget_from_id!(GameArea);

impl GameArea {
    pub fn get_game_area_state(&self) -> GameAreaState {
        GameAreaState {
            first_gen_was_drawn: self.game_state.first_gen_was_drawn,
            running:             self.game_state.running,
            single_step:         self.game_state.single_step,
            arrow_input:         self.game_state.arrow_input,
            drag_draw:           self.game_state.drag_draw,
            insert_mode:         self.insert_mode(),
        }
    }

    pub fn set_arrow_input(&mut self, input: (isize, isize)) {
        self.game_state.arrow_input = input;
    }

    pub fn set_drag_draw(&mut self, dd: Option<CellState>) {
        self.game_state.drag_draw = dd;
    }

    pub fn first_gen_drawn(&mut self) {
        self.game_state.first_gen_was_drawn = true;
    }

    pub fn insert_mode(&self) -> Option<(BitGrid, usize, usize)> {
        if let Some((bitgrid, row, col)) = &self.game_state.insert_mode {
            Some((bitgrid.clone(), *row, *col))
        } else {
            None
        }
    }
}

pub struct GameAreaState {
    pub first_gen_was_drawn: bool, // The purpose of this is to inhibit gen calc until the first draw
    pub running:             bool,
    // Input state
    pub single_step:         bool,
    pub arrow_input:         (isize, isize),
    pub drag_draw:           Option<CellState>,
    pub insert_mode:         Option<(BitGrid, usize, usize)>, // pattern to be drawn on click along with width and height;
}

impl Default for GameAreaState {
    fn default() -> Self {
        GameAreaState {
            first_gen_was_drawn: false,
            running:             false,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
            insert_mode:         None,
        }
    }
}
