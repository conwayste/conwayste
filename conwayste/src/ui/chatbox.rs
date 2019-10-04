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
use std::collections::VecDeque;

use ggez::graphics::{self, Color, DrawMode, DrawParam, FilterMode, Rect, Text};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{helpe::{within_widget, FontInfo}, widget::Widget, UIAction, WidgetID};

use crate::constants::{self, colors::*};

pub struct Chatbox {
    pub id: WidgetID,
    pub history_lines: usize,
    pub color: Color,
    pub messages: VecDeque<String>,
    pub wrapped: VecDeque<(bool, Text)>,
    pub dimensions: Rect,
    pub hover: bool,
    pub action: UIAction,
    font_info: FontInfo,
}

impl Chatbox {
    /// Creates a Chatbox widget.
    ///
    /// # Arguments
    /// * `widget_id` - Unique widget identifier
    /// * `font` - Font-type of chat text
    /// * `history_lines` - Number of lines of chat history to maintain
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::Chatbox;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Font::default();
    ///     let chatbox_rect = Rect::new(0.0, 0.0, chat_pane_rect.w, chat_pane_rect.h);
    ///     let mut chatbox = Chatbox::new(ui::InGamePane1Chatbox, font, 5);
    ///     chatbox.set_size(chatbox_rect);
    ///     chatbox.draw(ctx)?;
    /// }
    /// ```
    ///
    pub fn new(widget_id: WidgetID, font_info: FontInfo, history_lines: usize) -> Self {
        // TODO: affix to bottom left corner once "anchoring"/"gravity" is implemented
        let rect = *constants::DEFAULT_CHATBOX_RECT;
        Chatbox {
            id: widget_id,
            history_lines,
            color: *CHATBOX_BORDER_COLOR,
            messages: VecDeque::with_capacity(history_lines),
            wrapped: VecDeque::new(),
            dimensions: rect,
            hover: false,
            action: UIAction::EnterText,
            font_info,
        }
    }

    /// Adds a message to the chatbox
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `msg` - New chat message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ui::Chatbox;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Font::default();
    ///     let chatbox_rect = Rect::new(0.0, 0.0, chat_pane_rect.w, chat_pane_rect.h);
    ///     let mut chatbox = Chatbox::new(ui::InGamePane1Chatbox, font, 5);
    ///     chatbox.set_size(chatbox_rect);
    ///     chatbox.add_message(String::new("Player 1: This is a new chat message");
    ///     chatbox.add_message(String::new("-- This is a Server broadcast message -- ");
    ///     chatbox.draw(ctx)?;
    /// }
    /// ```
    ///
    pub fn add_message(&mut self, msg: String) -> GameResult<()> {
        self.messages.push_back(msg.clone());

        let mut texts = Chatbox::reflow_message(&msg, self.dimensions.w, &self.font_info);
        self.wrapped.append(&mut texts);

        // Remove any message(s) that exceed the alloted history. Any wrapped texts created from the
        // message(s) also need to be removed
        while self.messages.len() > self.history_lines {
            self.messages.pop_front();

            let mut count = 0;
            for (has_more, _) in self.wrapped.iter() {
                if *has_more {
                    count += 1;
                } else {
                    break;
                }
            }
            for _ in 0..count + 1 {
                self.wrapped.remove(0);
            }
        }

        Ok(())
    }

    fn reflow_messages(&mut self) {
        self.wrapped.clear();
        for msg in self.messages.iter_mut() {
            let mut texts = Chatbox::reflow_message(msg, self.dimensions.w, &self.font_info);
            self.wrapped.append(&mut texts);
        }
    }

    fn count_chars(msg: &str) -> usize {
        let mut count = 0;
        for _ in msg.chars() {
            count += 1;
        }
        count
    }

    fn reflow_message(msg: &str, width: f32, font_info: &FontInfo) -> VecDeque<(bool, Text)> {
        let mut texts = VecDeque::new();
        let max_chars_per_line = (width / font_info.char_dimensions.x) as usize;
        let mut s = String::with_capacity(max_chars_per_line);

        let mut chars_added = 0;
        for word in msg.split_whitespace() {

            // plus 1 to ensure we don't draw the last character of the word on the border
            if chars_added != 0 && chars_added + 1 + Chatbox::count_chars(word) > max_chars_per_line {
                let mut text = Text::new(s.clone());
                text.set_font(font_info.font, font_info.scale);
                texts.push_back((true, text));
                s.clear();
                chars_added = 0;
            }

            if chars_added == 0 && Chatbox::count_chars(word) > max_chars_per_line {
                // If word is longer than a line, then break the word into multiple lines
                for ch in word.chars() {
                    // plus 1 to ensure we don't draw the last character of the word on the border
                    if chars_added + 1 == max_chars_per_line {
                        let mut text = Text::new(s.clone());
                        text.set_font(font_info.font, font_info.scale);
                        texts.push_back((true, text));
                        s.clear();
                        chars_added = 0;
                    }

                    s.push(ch);
                    chars_added += 1;
                }
                // add a space after the long word and continue forward
                if !s.is_empty() {
                    s.push(' ');
                    chars_added += 1;
                }
                continue;
            }

            for ch in word.chars() {
                s.push(ch);
                chars_added += 1;
            }

            if chars_added + 1 <= max_chars_per_line {
                s.push(' ');
                chars_added += 1;
            }
        }

        if !s.is_empty() {
            let mut text = Text::new(s.clone());
            text.set_font(font_info.font, font_info.scale);
            texts.push_back((true, text));
        }

        if let Some((ref mut has_more_texts, _)) = texts.back_mut() {
            *has_more_texts = false;
        }

        texts
    }
}

impl Widget for Chatbox {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
        self.reflow_messages();
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        //if self.hover {
        //    debug!("Hovering over Chatbox, \"{:?}\"", self.label.dimensions);
        //}
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let hover = self.hover;
        self.hover = false;

        if hover {
            debug!("Clicked within Chatbox");
            return Some((self.id, self.action));
        }

        None
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        if self.hover {
            // Add in a teal border while hovered. Color checkbox differently to indicate hovered state.
            let border_rect = Rect::new(
                self.dimensions.x - 1.0,
                self.dimensions.y - 1.0,
                self.dimensions.w + constants::CHATBOX_BORDER_PIXELS / 2.0 + 2.0,
                self.dimensions.h + constants::CHATBOX_BORDER_PIXELS / 2.0 + 2.0,
            );
            let hovered_border = graphics::Mesh::new_rectangle(
                ctx,
                DrawMode::stroke(2.0),
                border_rect,
                *CHATBOX_BORDER_ON_HOVER_COLOR,
            )?;
            graphics::draw(ctx, &hovered_border, DrawParam::default())?;
        }

        let border = graphics::Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(constants::CHATBOX_BORDER_PIXELS),
            self.dimensions,
            self.color,
        )?;
        graphics::draw(ctx, &border, DrawParam::default())?;

        let mut max_lines = (self.dimensions.h / (self.font_info.char_dimensions.y + constants::CHATBOX_LINE_SPACING)) as u32;
        assert_ne!(max_lines, 0);

        let mut i = 0;
        let bottom_left_corner = Point2::new(self.dimensions.x, self.dimensions.y + self.dimensions.h - self.font_info.char_dimensions.y);
        for (_, wrapped_text) in self.wrapped.iter().rev() {
            if max_lines == 0 {
                break;
            }
            let point = Point2::new(
                bottom_left_corner.x + constants::CHATBOX_BORDER_PIXELS + 1.0,
                bottom_left_corner.y - (i as f32 * self.font_info.char_dimensions.y)
            );
            graphics::queue_text(ctx, wrapped_text, point, Some(*CHATBOX_TEXT_COLOR));
            max_lines -= 1;
            i += 1;
        }

        graphics::draw_queued_text(ctx, DrawParam::default(), None, FilterMode::Linear)?;

        Ok(())
    }
}
