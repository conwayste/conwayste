
/*  Copyright 2019 the Conwayste Developers.
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
use chromatica::css;

use std::rc::Rc;
use std::time::{Instant, Duration};

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam, Text};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    helpe::{within_widget, draw_text, color_with_alpha},
    UIAction, WidgetID
};

use crate::constants::DEFAULT_UI_FONT_SCALE;

pub const TEXT_INPUT_BUFFER_LEN     : usize = 255;
pub const BLINK_RATE_MS             : u64 = 500;

const AVERAGE_CHARACTER_WIDTH_PX     : f32 = 9.0; // in pixels

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TextInputState {
    EnteringText,
    TextInputComplete,
}

pub struct TextField {
    pub id: WidgetID,
    pub action: UIAction,
    pub state: Option<TextInputState>, // PR_GATE input state
    text: String,
    pub cursor_index: usize,
    pub blink_timestamp: Option<Instant>,
    pub draw_cursor: bool,
    pub dimensions: Rect,
    pub hover: bool,
    pub visible_start_index: usize,
    font: Rc<Font>,
}

/// A widget that can accept and display user-inputted text from the Keyboard.
impl TextField {
    /// Creates a TextField widget.
    ///
    /// # Arguments
    /// * `widget_id` - Unique widget identifier
    /// * `font` - font-type for drawn text
    /// * `dimensions` - rectangle describing the size of the text field
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ui::TextField;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Rc::new(Font::default());
    ///     let dimensions = Rect::new(0.0, 0.0, 300.0, 20.0);
    ///
    ///     let textfield = TextField::new(ui::ChatboxTextField, font, dimensions);
    ///
    ///     textfield.draw(ctx)?;
    /// }
    /// ```
    ///
    pub fn new(widget_id: WidgetID, font: Rc<Font>, dimensions: Rect) -> TextField {
        TextField {
            state: None,
            text: String::with_capacity(TEXT_INPUT_BUFFER_LEN),
            cursor_index: 0,
            blink_timestamp: None,
            draw_cursor: false,
            dimensions: dimensions,
            id: widget_id,
            action: UIAction::EnterText,
            hover: false,
            visible_start_index: 0,
            font: font,
        }
    }

    /// Returns the a string of the inputted text
    pub fn text(&self) -> Option<String> {
        let trimmed_str = self.text.trim();
        if !trimmed_str.is_empty() {
            return Some(String::from(trimmed_str));
        }
        None
    }

    /// Sets the text field's string contents
    pub fn _set_text(&mut self, text: String) {
        self.text = text;
        self.cursor_index = 0;
    }

    fn get_text_width_in_px(&self, ctx: &mut Context) -> f32 {
        let mut text = Text::new(self.text.clone());
        let text = text.set_font(*self.font, *DEFAULT_UI_FONT_SCALE);
        text.width(ctx) as f32
    }

    /// Adds a character at the current cursor position
    pub fn add_char_at_cursor(&mut self, _ctx: &mut Context, character: char)
    {
        if self.cursor_index == self.text.len() {
            self.text.push(character);
        } else {
            self.text.insert(self.cursor_index, character);
        }
        self.cursor_index += 1;
    }

    /// Deletes a character to the left of the current cursor
    pub fn remove_left_of_cursor(&mut self, _ctx: &mut Context) {
        if self.cursor_index != 0 {
            if self.cursor_index == self.text.len() {
                self.text.pop();
            } else {
                self.text.remove(self.cursor_index);
            }
            self.cursor_index -= 1;
        }
    }

    /// Deletes a chracter to the right of the current cursor
    pub fn remove_right_of_cursor(&mut self, _ctx: &mut Context) {
        let text_len = self.text.len();

        if text_len != 0 && self.cursor_index != text_len {
            self.text.remove(self.cursor_index);
        }
    }

    /// Clears the contents of the text field
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_index = 0;
        self.visible_start_index = 0;
        self.blink_timestamp = None;
        self.draw_cursor = false;
    }

    /// Advances the cursor position to the right by one character
    pub fn move_cursor_right(&mut self, _ctx: &mut Context) {
        if self.cursor_index < self.text.len() {
            self.cursor_index += 1;

            let last_visible_index = DEFAULT_UI_FONT_SCALE.x as usize;
            if self.cursor_index - self.visible_start_index > last_visible_index + 1 {
                self.visible_start_index += 1;
                println!("move cursor right moves start index")
            }
        }
    }

    /// Decrements the cursor position to the left by one character
    pub fn move_cursor_left(&mut self, _ctx: &mut Context) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;

            if self.visible_start_index != 0 && self.cursor_index == self.visible_start_index {
                self.visible_start_index -= 1;
            }
        }
    }

    /// Moves the cursor prior to the first character in the field
    pub fn cursor_home(&mut self) {
        self.cursor_index = 0;
        self.visible_start_index = 0;
    }

    /// Moves the cursor after the last character in the field
    pub fn cursor_end(&mut self, ctx: &mut Context) {
        let text_length = self.text.len();
        self.cursor_index = text_length;

        // TODO Reverify functionality once https://github.com/ggez/ggez/issues/583 is fixed.
        //      We should then just be able to use DEFAULT_UI_FONT_SCALE.x to calculate the length
        //      and remove the need to pass down context to get the width.
        let text_width_px = self.get_text_width_in_px(ctx);
        if text_width_px > self.dimensions.w {
            let avg_character_length = text_width_px / self.text.len() as f32;
            let index = (text_width_px - self.dimensions.w)/avg_character_length;

            // Add one to ensure visible text remains fully bounded by self.dimensions.w
            self.visible_start_index = index as usize + 1;
        }
    }

    /// Textfield gains focus and begins accepting user input
    pub fn enter_focus(&mut self) {
        self.state = Some(TextInputState::EnteringText);
        self.blink_timestamp = Some(Instant::now());
    }

    /// Textfield loses focus and does not accept user input
    pub fn exit_focus(&mut self) {
        self.state = None;
        self.draw_cursor = false;
    }
}

impl Widget for TextField {
    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let hover = self.hover;
        self.hover = false;

        if hover {
            self.enter_focus();
            return Some( (self.id, self.action) );
        }
        None
    }

    fn on_drag(&mut self, _original_point: &Point2<f32>, _point: &Point2<f32>) {
        // Any point to implementing highlighting in 1.0?
        ()
    }

    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        if Some(TextInputState::EnteringText) == self.state {
            if let Some(prev_blink_ms) = self.blink_timestamp {
                if Instant::now() - prev_blink_ms > Duration::from_millis(BLINK_RATE_MS) {
                    self.draw_cursor ^= true;
                    self.blink_timestamp = Some(Instant::now());
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // PR_GATE: If string exceeds length of pane, need to only draw what should be visible

        if self.state.is_some() || !self.text.is_empty() {
            const CURSOR_OFFSET_PX: f32 = 5.0;

            let colored_rect;
            if !self.text.is_empty() && self.state.is_none() {
                colored_rect = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(4.0), self.dimensions, color_with_alpha(css::VIOLET, 0.5))?;
            } else {
                colored_rect = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(4.0), self.dimensions, Color::from(css::VIOLET))?;
            }

            graphics::draw(ctx, &colored_rect, DrawParam::default())?;

            let text_with_cursor = self.text.clone();
            let mut text_pos = Point2::new(self.dimensions.x + CURSOR_OFFSET_PX, self.dimensions.y + 3.0);

            // PR_GATE fix how this works overall now that we have fixed width fonts
            let visible_text;
            if (self.text.len() - self.visible_start_index) as f32 > self.dimensions.w {
                let end_index = self.visible_start_index as f32 + self.dimensions.w/DEFAULT_UI_FONT_SCALE.x - 1.0;
                visible_text = text_with_cursor[self.visible_start_index..end_index as usize].to_owned();
            } else {
                visible_text = text_with_cursor[self.visible_start_index..self.text.len()].to_owned();
            }

            draw_text(ctx, Rc::clone(&self.font), Color::from(css::WHITESMOKE), visible_text, &text_pos)?;

            if self.draw_cursor {
                let mut text = Text::new(&text_with_cursor[self.visible_start_index..self.cursor_index]);
                let text = text.set_font(*self.font, *DEFAULT_UI_FONT_SCALE);
                let cursor_position_px = text.width(ctx) as f32;

                text_pos.x += cursor_position_px;
                draw_text(ctx, Rc::clone(&self.font), Color::from(css::WHITESMOKE), String::from("|"), &text_pos)?;
            }
        }

        Ok(())
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dimensions: Rect) {
        self.dimensions = new_dimensions;
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn id(&self) -> WidgetID {
        self.id
    }
}
