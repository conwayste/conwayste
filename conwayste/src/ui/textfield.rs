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

use std::time::{Duration, Instant};

use ggez::graphics::{self, DrawMode, DrawParam, Rect};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    common::{within_widget, FontInfo},
    widget::Widget,
    UIAction, WidgetID,
};
#[cfg(not(test))]
use super::common::draw_text;

use crate::constants::{colors::*, CHATBOX_BORDER_PIXELS};

pub const BLINK_RATE_MS: u64 = 500;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TextInputState {
    EnteringText,
    TextInputComplete,
}

pub struct TextField {
    id: WidgetID,
    action: UIAction,
    pub input_state: Option<TextInputState>,
    text: String,
    cursor_index: usize, // Position of the cursor: 0 means before first character; it's at the end when equal to text.len()
    // `blink_timestamp` and `draw_cursor` are used to control the blinking of the cursor.
    blink_timestamp: Option<Instant>,
    draw_cursor: bool,
    dimensions: Rect,
    hover: bool,
    visible_start_index: usize, // The index of the first character in `self.text` that is visible.
    font_info: FontInfo,
}

/// A widget that can accept and display user-inputted text from the Keyboard.
impl TextField {
    /// Creates a TextField widget.
    ///
    /// # Arguments
    /// * `widget_id` - Unique widget identifier
    /// * `font_info` - font descriptor to be used when drawing the text
    /// * `dimensions` - rectangle describing the size of the text field
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::{self, TextField};
    ///
    /// let font = Font::Default;
    /// let font_info = common::FontInfo::new(ctx, font, Some(20.0));
    /// let dimensions = Rect::new(0.0, 0.0, 300.0, 20.0);
    ///
    /// let textfield = TextField::new(ui::ChatboxTextField, font_info, dimensions);
    ///
    /// textfield.draw(ctx);
    /// ```
    ///
    pub fn new(widget_id: WidgetID, font_info: FontInfo, dimensions: Rect) -> TextField {
        TextField {
            input_state: None,
            text: String::new(),
            cursor_index: 0,
            blink_timestamp: None,
            draw_cursor: false,
            dimensions: dimensions,
            id: widget_id,
            action: UIAction::EnterText,
            hover: false,
            visible_start_index: 0,
            font_info,
        }
    }

    /// Maximum number of characters that can be visible at once. Computed from `dimensions` and `single_char_width`.
    fn max_visible_chars(&self) -> usize {
        (self.dimensions.w / self.font_info.char_dimensions.x) as usize
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

    /// Adds a character at the current cursor position
    pub fn add_char_at_cursor(&mut self, character: char) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        if self.cursor_index == self.text.len() {
            self.text.push(character);
        } else {
            self.text.insert(self.cursor_index, character);
        }
        self.cursor_index += 1;
        if self.visible_start_index + self.max_visible_chars() < self.cursor_index {
            self.visible_start_index = self.cursor_index - self.max_visible_chars();
        }
    }

    /// Deletes a character to the left of the current cursor
    pub fn remove_left_of_cursor(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        if self.cursor_index != 0 {
            self.text.remove(self.cursor_index - 1);
            self.cursor_index -= 1;
            if self.visible_start_index > self.cursor_index {
                self.visible_start_index = self.cursor_index;
            }
        }
    }

    /// Deletes a chracter to the right of the current cursor
    pub fn remove_right_of_cursor(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

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

    /// Moves the cursor position to the right by one character
    pub fn move_cursor_right(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        if self.cursor_index < self.text.len() {
            self.cursor_index += 1;

            if self.visible_start_index + self.max_visible_chars() < self.cursor_index {
                self.visible_start_index = self.cursor_index - self.max_visible_chars();
            }
        }
    }

    /// Moves the cursor position to the left by one character
    pub fn move_cursor_left(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        if self.cursor_index > 0 {
            self.cursor_index -= 1;

            if self.visible_start_index > self.cursor_index {
                self.visible_start_index = self.cursor_index;
            }
        }
    }

    /// Moves the cursor before to the first character in the field
    pub fn cursor_home(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        self.cursor_index = 0;
        self.visible_start_index = 0;
    }

    /// Moves the cursor after the last character in the field
    pub fn cursor_end(&mut self) {
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());

        self.cursor_index = self.text.len();
        if self.text.len() - self.visible_start_index > self.max_visible_chars() {
            self.visible_start_index = self.text.len() - self.max_visible_chars();
        }
    }

    /// Textfield gains focus and begins accepting user input
    pub fn enter_focus(&mut self) {
        self.input_state = Some(TextInputState::EnteringText);
        self.draw_cursor = true;
        self.blink_timestamp = Some(Instant::now());
    }

    /// Textfield loses focus and does not accept user input
    pub fn exit_focus(&mut self) {
        self.input_state = None;
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
            return Some((self.id, self.action));
        }
        None
    }

    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        if Some(TextInputState::EnteringText) == self.input_state {
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
        if self.input_state.is_some() || !self.text.is_empty() {
            let colored_rect;
            if !self.text.is_empty() && self.input_state.is_none() {
                colored_rect = graphics::Mesh::new_rectangle(
                    ctx,
                    DrawMode::stroke(CHATBOX_BORDER_PIXELS),
                    self.dimensions,
                    *CHATBOX_INACTIVE_BORDER_COLOR,
                )?;
            } else {
                colored_rect = graphics::Mesh::new_rectangle(
                    ctx,
                    DrawMode::stroke(CHATBOX_BORDER_PIXELS),
                    self.dimensions,
                    *CHATBOX_BORDER_COLOR,
                )?;
            }

            graphics::draw(ctx, &colored_rect, DrawParam::default())?;

            // 3.0 px added to y for central alignment
            let text_pos = Point2::new(
                self.dimensions.x + CHATBOX_BORDER_PIXELS / 2.0 + 1.0,
                self.dimensions.y + 3.0,
            );

            let mut end = self.text.len();
            if self.visible_start_index + self.max_visible_chars() < end {
                end = self.visible_start_index + self.max_visible_chars();
            }
            let visible_text = self.text[self.visible_start_index..end].to_owned();

            #[cfg(not(test))]
            {
                draw_text(
                    ctx,
                    self.font_info.font,
                    *INPUT_TEXT_COLOR,
                    visible_text,
                    &text_pos,
                )?;
            }
            #[cfg(test)]
            {
                let _ = visible_text;  // suppress warning
            }

            if self.draw_cursor {
                let mut cursor_pos = text_pos.clone();

                cursor_pos.x += (self.cursor_index - self.visible_start_index) as f32
                    * self.font_info.char_dimensions.x;

                // Remove half the width of a character so the pipe character is at the beginning
                // of its area (like a cursor), not the center (like a character).
                cursor_pos.x -= self.font_info.char_dimensions.x / 2.0;

                #[cfg(not(test))]
                {
                    draw_text(
                        ctx,
                        self.font_info.font,
                        *INPUT_TEXT_COLOR,
                        String::from("|"),
                        &cursor_pos,
                    )?;
                }
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

widget_from_id!(TextField);
