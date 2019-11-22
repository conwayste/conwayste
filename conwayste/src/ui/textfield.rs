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

use ggez::event::KeyCode;
use ggez::graphics::{self, Color, DrawMode, DrawParam, Rect};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

#[cfg(not(test))]
use super::common::draw_text;
use super::{
    common::{within_widget, FontInfo},
    widget::Widget,
    UIAction, UIError, UIResult, WidgetID,
};

use crate::constants::{colors::*, CHATBOX_BORDER_PIXELS};

pub const BLINK_RATE_MS: u64 = 500;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TextInputState {
    EnteringText,
    TextInputComplete,
}

pub struct TextField {
    id: WidgetID,
    z_index: usize,
    action: UIAction,
    pub input_state: Option<TextInputState>,
    text: String,
    cursor_index: usize, // Position of the cursor in the text fields' string
    cursor_blink_timestamp: Option<Instant>, // last time the cursor blinked on/off
    draw_cursor: bool,
    dimensions: Rect,
    hover: bool,
    visible_start_index: usize, // The index of the first character in `self.text` that is visible.
    font_info: FontInfo,
    pub bg_color: Option<Color>,
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
            id: widget_id,
            z_index: 0,
            input_state: None,
            text: String::new(),
            cursor_index: 0,
            cursor_blink_timestamp: None,
            draw_cursor: false,
            dimensions: dimensions,
            action: UIAction::EnterText,
            hover: false,
            visible_start_index: 0,
            font_info,
            bg_color: None,
        }
    }

    /// Maximum number of characters that can be visible at once.
    /// Computed from `dimensions` and `single_char_width`.
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

    /// Handle a text character being typed by the user.
    pub fn on_char(&mut self, character: char) {
        if self.input_state.is_some() {
            self.add_char_at_cursor(character);
        }
    }

    /// Handle a typed keycode that is not a textual key. For example, arrow keys.
    pub fn on_keycode(&mut self, keycode: KeyCode) {
        if keycode == KeyCode::Return {
            self.input_state = Some(TextInputState::TextInputComplete);
            return;
        }
        if let Some(TextInputState::EnteringText) = self.input_state {
            match keycode {
                KeyCode::Back => self.remove_left_of_cursor(),
                KeyCode::Delete => self.remove_right_of_cursor(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Home => self.cursor_home(),
                KeyCode::End => self.cursor_end(),
                KeyCode::Escape => self.exit_focus(),
                _ => ()
            }
        }
    }

    /// Adds a character at the current cursor position
    fn add_char_at_cursor(&mut self, character: char) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

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
    fn remove_left_of_cursor(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

        if self.cursor_index != 0 {
            self.text.remove(self.cursor_index - 1);
            self.cursor_index -= 1;
            if self.visible_start_index > self.cursor_index {
                self.visible_start_index = self.cursor_index;
            }
        }
    }

    /// Deletes a chracter to the right of the current cursor
    fn remove_right_of_cursor(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

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
        self.cursor_blink_timestamp = None;
        self.draw_cursor = false;
    }

    /// Moves the cursor position to the right by one character
    fn move_cursor_right(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

        if self.cursor_index < self.text.len() {
            self.cursor_index += 1;

            if self.visible_start_index + self.max_visible_chars() < self.cursor_index {
                self.visible_start_index = self.cursor_index - self.max_visible_chars();
            }
        }
    }

    /// Moves the cursor position to the left by one character
    fn move_cursor_left(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

        if self.cursor_index > 0 {
            self.cursor_index -= 1;

            if self.visible_start_index > self.cursor_index {
                self.visible_start_index = self.cursor_index;
            }
        }
    }

    /// Moves the cursor before to the first character in the field
    fn cursor_home(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

        self.cursor_index = 0;
        self.visible_start_index = 0;
    }

    /// Moves the cursor after the last character in the field
    fn cursor_end(&mut self) {
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());

        self.cursor_index = self.text.len();
        if self.text.len() - self.visible_start_index > self.max_visible_chars() {
            self.visible_start_index = self.text.len() - self.max_visible_chars();
        }
    }
}

impl Widget for TextField {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn z_index(&self) -> usize {
        self.z_index
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        self.enter_focus();
        return Some((self.id, self.action));
    }

    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        if Some(TextInputState::EnteringText) == self.input_state {
            if let Some(prev_blink_ms) = self.cursor_blink_timestamp {
                if Instant::now() - prev_blink_ms > Duration::from_millis(BLINK_RATE_MS) {
                    self.draw_cursor ^= true;
                    self.cursor_blink_timestamp = Some(Instant::now());
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.input_state.is_none() && self.text.is_empty() {
            // textfield is hidden
            return Ok(());
        }

        if let Some(bg_color) = self.bg_color {
            let mesh =
                graphics::Mesh::new_rectangle(ctx, DrawMode::fill(), self.dimensions, bg_color)?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

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
            let _ = visible_text; // suppress warning
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

        Ok(())
    }

    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the size of a TextField {:?} to a width or height of
                    zero", self.id()),
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
                reason: format!("Cannot set the width or height of Label {:?} to zero", self.id())
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    /// Textfield gains focus and begins accepting user input
    fn enter_focus(&mut self) {
        self.input_state = Some(TextInputState::EnteringText);
        self.draw_cursor = true;
        self.cursor_blink_timestamp = Some(Instant::now());
    }

    /// Textfield loses focus and does not accept user input
    fn exit_focus(&mut self) {
        self.input_state = None;
        self.draw_cursor = false;
    }
}

widget_from_id!(TextField);

#[cfg(test)]
mod test {
    use super::*;
    use ggez::graphics::Scale;

    fn create_dummy_textfield() -> TextField {
        let font_info = FontInfo {
            font: (),                   //dummy font because we can't create a real Font without ggez
            scale: Scale::uniform(1.0), // I don't think this matters
            char_dimensions: Vector2::<f32>::new(5.0, 5.0), // any positive values will do
        };
        TextField::new(WidgetID(1), font_info, Rect::new(0.0, 0.0, 100.0, 100.0))
    }

    #[test]
    fn test_add_char_at_cursor_beginning_middle_end() {
        let mut tf = create_dummy_textfield();

        assert_eq!(tf.cursor_index, 0);

        tf.add_char_at_cursor('A');
        assert_eq!(tf.cursor_index, 1);

        tf.add_char_at_cursor('B');
        assert_eq!(tf.cursor_index, 2);

        tf.move_cursor_left();
        assert_eq!(tf.cursor_index, 1);

        tf.add_char_at_cursor('C');
        assert_eq!(tf.cursor_index, 2);
    }

    #[test]
    fn test_add_char_at_cursor_exceeds_dimensions() {
        let mut tf = create_dummy_textfield();
        let max_chars = tf.max_visible_chars();

        for _ in 0..max_chars + 2 {
            tf.add_char_at_cursor('A');
        }

        assert_eq!(tf.visible_start_index, 2);
    }

    #[test]
    fn test_move_cursor_left_at_limits() {
        let mut tf = create_dummy_textfield();
        assert_eq!(tf.cursor_index, 0);
        tf.move_cursor_left();
        assert_eq!(tf.cursor_index, 0);

        let test_string = "TestString";
        for ch in test_string.chars() {
            tf.add_char_at_cursor(ch);
        }

        tf.move_cursor_left();
        assert_eq!(tf.cursor_index, test_string.len() - 1);
        tf.move_cursor_left();
        assert_eq!(tf.cursor_index, test_string.len() - 2);
    }

    #[test]
    fn test_move_cursor_right_at_limits() {
        let mut tf = create_dummy_textfield();
        assert_eq!(tf.cursor_index, 0);
        tf.move_cursor_right();
        assert_eq!(tf.cursor_index, 0);

        let test_string = "TestString";
        for ch in test_string.chars() {
            tf.add_char_at_cursor(ch);
        }

        tf.move_cursor_right();
        assert_eq!(tf.cursor_index, test_string.len());
        tf.move_cursor_right();
        assert_eq!(tf.cursor_index, test_string.len());
        tf.move_cursor_left();
        tf.move_cursor_right();
        assert_eq!(tf.cursor_index, test_string.len());
    }

    #[test]
    fn test_move_cursor_to_home() {
        let mut tf = create_dummy_textfield();
        assert_eq!(tf.cursor_index, 0);

        let test_string = "TestString";
        for ch in test_string.chars() {
            tf.add_char_at_cursor(ch);
        }
        assert_eq!(tf.cursor_index, test_string.len());
        tf.cursor_home();
        assert_eq!(tf.cursor_index, 0);
    }

    #[test]
    fn test_move_cursor_to_end() {
        let mut tf = create_dummy_textfield();
        assert_eq!(tf.cursor_index, 0);

        let test_string = "TestString";
        for ch in test_string.chars() {
            tf.add_char_at_cursor(ch);
        }
        assert_eq!(tf.cursor_index, test_string.len());
        tf.cursor_home();
        assert_eq!(tf.cursor_index, 0);
        tf.cursor_end();
        assert_eq!(tf.cursor_index, test_string.len());
    }

    #[test]
    fn test_move_cursor_left_when_string_exceeds_limits() {
        let mut tf = create_dummy_textfield();
        let max_chars = tf.max_visible_chars();

        for _ in 0..max_chars + 2 {
            tf.add_char_at_cursor('A');
        }

        for _ in 0..max_chars + 1 {
            assert_eq!(tf.visible_start_index, 2);
            tf.move_cursor_left();
        }
        assert_eq!(tf.visible_start_index, 1);
        tf.move_cursor_left();
        assert_eq!(tf.visible_start_index, 0);
    }

    #[test]
    fn test_move_cursor_right_when_string_exceeds_limits() {
        let mut tf = create_dummy_textfield();
        let max_chars = tf.max_visible_chars();

        for _ in 0..max_chars + 2 {
            tf.add_char_at_cursor('A');
        }

        tf.cursor_home();

        for _ in 0..max_chars + 1 {
            assert_eq!(tf.visible_start_index, 0);
            tf.move_cursor_right();
        }
        assert_eq!(tf.visible_start_index, 1);
        tf.move_cursor_right();
        assert_eq!(tf.visible_start_index, 2);
    }

    #[test]
    fn test_remove_left_of_cursor_basic_case() {
        let mut tf = create_dummy_textfield();

        assert_eq!(tf.text, "");
        tf.remove_left_of_cursor();
        assert_eq!(tf.text, "");

        for _ in 0..10 {
            tf.add_char_at_cursor('A');
        }
        assert_eq!(tf.text, "AAAAAAAAAA");

        for _ in 0..10 {
            tf.remove_left_of_cursor();
        }
        assert_eq!(tf.text, "");
    }

    #[test]
    fn test_remove_left_of_cursor_string_exceeds_limits_and_remove_contents() {
        let mut tf = create_dummy_textfield();
        let max_chars = tf.max_visible_chars();

        for _ in 0..max_chars + 2 {
            tf.add_char_at_cursor('A');
        }
        assert_eq!(tf.text, "AAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(tf.visible_start_index, 2);
        tf.remove_left_of_cursor();
        assert_eq!(tf.visible_start_index, 2);
        tf.remove_left_of_cursor();
        assert_eq!(tf.visible_start_index, 2);
        tf.remove_left_of_cursor();
        assert_eq!(tf.visible_start_index, 2);

        for _ in 0..max_chars - 2 {
            tf.remove_left_of_cursor();
        }
        assert_eq!(tf.visible_start_index, 1);
        tf.remove_left_of_cursor();
        assert_eq!(tf.visible_start_index, 0);

        assert_eq!(tf.text, "");
    }

    #[test]
    fn test_remove_right_of_cursor_basic_case() {
        let mut tf = create_dummy_textfield();

        assert_eq!(tf.text, "");
        tf.remove_right_of_cursor();
        assert_eq!(tf.text, "");

        for _ in 0..10 {
            tf.add_char_at_cursor('A');
        }
        assert_eq!(tf.text, "AAAAAAAAAA");
        tf.remove_right_of_cursor();
        assert_eq!(tf.text, "AAAAAAAAAA");

        tf.cursor_home();

        for _ in 0..10 {
            tf.remove_right_of_cursor();
        }
        assert_eq!(tf.text, "");
    }

    #[test]
    fn test_remove_right_of_cursor_does_not_impact_visible_index() {
        let mut tf = create_dummy_textfield();
        let max_chars = tf.max_visible_chars();

        for _ in 0..max_chars + 2 {
            tf.add_char_at_cursor('A');
        }
        assert_eq!(tf.text, "AAAAAAAAAAAAAAAAAAAAAA");
        tf.cursor_home();

        for _ in 0..tf.text.len() {
            assert_eq!(tf.visible_start_index, 0);
            tf.remove_right_of_cursor();
        }
        tf.remove_right_of_cursor();
        assert_eq!(tf.visible_start_index, 0);

        assert_eq!(tf.text, "");
    }
}
