
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

use std::time::{Instant, Duration};

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam, Text, Scale};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    helpe::{within_widget, draw_text, color_with_alpha},
    UIAction, WidgetID
};

pub const TEXT_INPUT_BUFFER_LEN     : usize = 255;
pub const BLINK_RATE_MS             : u64 = 500;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TextInputState {
    EnteringText,
    TextInputComplete,
}

pub struct TextField {
    pub id: WidgetID,
    pub action: UIAction,
    pub state: Option<TextInputState>, // fixme input state
    text: String,
    pub cursor_index: usize,
    pub blink_timestamp: Option<Instant>,
    pub draw_cursor: bool,
    pub dimensions: Rect,
    pub hover: bool,
}

impl TextField {
    pub fn new((x,y): (f32, f32), widget_id: WidgetID) -> TextField {
        TextField {
            state: None,
            text: String::with_capacity(TEXT_INPUT_BUFFER_LEN),
            cursor_index: 0,
            blink_timestamp: None,
            draw_cursor: false,
            dimensions: Rect::new(x, y, 300.0, 30.0),
            id: widget_id,
            action: UIAction::EnterText,
            hover: false,
        }
    }

    pub fn text(&self) -> Option<String> {
        let trimmed_str = self.text.trim();
        if !trimmed_str.is_empty() {
            return Some(String::from(trimmed_str));
        }
        None
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor_index = 0;
    }

    pub fn add_char_at_cursor(&mut self, character: char)
    {
        if self.cursor_index == self.text.len() {
            self.text.push(character);
        } else {
            self.text.insert(self.cursor_index, character);
        }
        self.cursor_index += 1;
    }

    pub fn add_string_at_cursor(&mut self, text: String) {
        if self.cursor_index == self.text.len() {
            self.text.push_str(&text);
        } else {
            self.text.insert_str(self.cursor_index, &text);
        }
        self.cursor_index += text.len();
    }

    /// Deletes a character to the left of the current cursor
    pub fn remove_left_of_cursor(&mut self) {
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
    pub fn remove_right_of_cursor(&mut self) {
        if self.text.len() != 0 && self.cursor_index != self.text.len() {
            self.text.remove(self.cursor_index);
        }
    }

    /// Clears the contents of the text field
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_index = 0;
        self.blink_timestamp = None;
        self.draw_cursor = false;
    }

    pub fn inc_cursor_pos(&mut self) {
        if self.cursor_index < self.text.len() {
            self.cursor_index += 1;
        }
    }

    pub fn dec_cursor_pos(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.cursor_index = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor_index = self.text.len();
    }

    pub fn enter_focus(&mut self) {
        self.state = Some(TextInputState::EnteringText);
        self.blink_timestamp = Some(Instant::now());
    }

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

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        // TODO: If string exceeds length of pane, need to only draw what should be visible

        if self.state.is_some() || !self.text.is_empty() {
            const CURSOR_OFFSET_PX: f32 = 10.0;

            let colored_rect;
            if !self.text.is_empty() && self.state.is_none() {
                colored_rect = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(4.0), self.dimensions, color_with_alpha(css::VIOLET, 0.5))?;
            } else {
                colored_rect = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(4.0), self.dimensions, Color::from(css::VIOLET))?;
            }

            graphics::draw(ctx, &colored_rect, DrawParam::default())?;

            let text_with_cursor = self.text.clone();
            let text_pos = Point2::new(self.dimensions.x + CURSOR_OFFSET_PX, self.dimensions.y);

            draw_text(ctx, font, Color::from(css::WHITESMOKE), &text_with_cursor, &text_pos, None)?;

            if self.draw_cursor {
                let mut text = Text::new(&text_with_cursor[0..self.cursor_index]);
                let text = text.set_font(*font, Scale::uniform(20.0));
                let cursor_position_px = text.width(ctx) as f32;
                let cursor_position = Point2::new(self.dimensions.x + cursor_position_px + CURSOR_OFFSET_PX, self.dimensions.y);
                draw_text(ctx, font, Color::from(css::WHITESMOKE), "|", &cursor_position, None)?;
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

    fn translate(&mut self, point: Vector2<f32>) {
        self.dimensions.translate(point);
    }

    fn id(&self) -> WidgetID {
        self.id
    }
}