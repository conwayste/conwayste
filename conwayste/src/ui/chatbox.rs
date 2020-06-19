/*  Copyright 2019-2020 the Conwayste Developers.
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

use std::error::Error;
use std::collections::VecDeque;
use std::fmt;
use std::sync::mpsc::{Receiver, Sender, channel};

use ggez::graphics::{self, Color, DrawMode, DrawParam, FilterMode, Rect, Text};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use id_tree::NodeId;

use super::{
    common::{within_widget, FontInfo},
    widget::Widget,
    UIError, UIResult,
    context::{HandlerData, EmitEvent, EventType, Handled, Event, UIContext},
};

use crate::constants::{self, colors::*};

pub struct Chatbox {
    id: Option<NodeId>,
    z_index: usize,
    history_lines: usize,
    color: Color,
    messages: VecDeque<String>,
    wrapped: VecDeque<(bool, Text)>,
    dimensions: Rect,
    hover: bool,
    font_info: FontInfo,
    msg_sender: Sender<String>,
    msg_receiver: Receiver<String>,
    handler_data: HandlerData,
}

impl fmt::Debug for Chatbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chatbox {{ id: {:?}, z_index: {}, dimensions: {:?}, history_lines: {} }}",
            self.id, self.z_index, self.dimensions, self.history_lines)
    }
}

impl Chatbox {
    /// Creates a Chatbox widget.
    ///
    /// # Arguments
    /// * `font_info` - a `FontInfo` struct to represent that chat text's font
    /// * `history_lines` - Number of lines of chat history to maintain
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::{self, Checkbox, common};
    ///
    /// let font = Font::Default;
    /// let chatbox_font_info = common::FontInfo::new(ctx, font, Some(20.0));
    /// let chatbox = Chatbox::new(chatbox_font_info, 20);
    /// checkbox.draw(ctx);
    /// ```
    ///
    pub fn new(font_info: FontInfo, history_lines: usize) -> Self {
        // TODO: affix to bottom left corner once "anchoring"/"gravity" is implemented
        let rect = *constants::DEFAULT_CHATBOX_RECT;
        let (msg_tx, msg_rx) = channel::<String>();
        let mut chatbox = Chatbox {
            id: None,
            z_index: std::usize::MAX,
            history_lines,
            color: *CHATBOX_BORDER_COLOR,
            messages: VecDeque::with_capacity(history_lines),
            wrapped: VecDeque::new(),
            dimensions: rect,
            hover: false,
            font_info,
            msg_sender: msg_tx,
            msg_receiver: msg_rx,
            handler_data: HandlerData::new(),
        };
        chatbox.on(EventType::Update, Box::new(Chatbox::update_handler)).unwrap(); // unwrap OK because we aren't in handler
        chatbox
    }

    /// Returns a handle that enables you to asynchronously publish messages to this chatbox.
    pub fn new_handle(&self) -> ChatboxPublishHandle {
        ChatboxPublishHandle::new(self.msg_sender.clone())
    }

    fn update_handler(obj: &mut dyn EmitEvent, _uictx: &mut UIContext, _evt: &Event) -> Result<Handled, Box<dyn Error>> {
        let chatbox = obj.downcast_mut::<Chatbox>().unwrap(); // unwrap OK because it's always a Chatbox
        loop {
            if let Ok(msg) = chatbox.msg_receiver.try_recv() {
                // TODO: maybe we should batch add these? Benchmark!
                chatbox.add_message(msg);
            } else {
                break;
            }
        }
        Ok(Handled::NotHandled)
    }

    /// Adds a message to the chatbox
    ///
    /// # Arguments
    /// * `msg` - New chat message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::{Chatbox, common};
    ///
    /// let font = Font::Default;
    /// let font_info = common::FontInfo::new(ctx, font, Some(20.0));
    /// let mut chatbox = Chatbox::new(SOME_CHATBOX_WIDGET_ID, font_info, 200);
    /// chatbox.add_message(String::new("Player 1: This is a new chat message");
    /// chatbox.add_message(String::new("-- This is a Server broadcast message -- ");
    /// chatbox.set_rect(chatbox_rect);
    /// chatpane.add(Box::new(chatbox));
    ///
    /// //...
    /// chatbox.draw(ctx);
    /// ```
    ///
    pub fn add_message(&mut self, msg: String) {
        let mut texts = Chatbox::reflow_message(&msg, self.dimensions.w, &self.font_info);
        self.wrapped.append(&mut texts);

        self.messages.push_back(msg);

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

    /// Breaks the message up into segments that are at most `width` long for the provided `font_info`
    fn reflow_message(msg: &str, width: f32, font_info: &FontInfo) -> VecDeque<(bool, Text)> {
        let mut texts = VecDeque::new();
        let max_chars_per_line = (width / font_info.char_dimensions.x) as usize;
        let mut s = String::with_capacity(max_chars_per_line);

        let mut chars_added = 0;
        for word in msg.split_whitespace() {
            let word_chars = Chatbox::count_chars(word);

            // If the word can fit on the next line, but not the current line
            if chars_added != 0
                && chars_added + word_chars > max_chars_per_line
                && word_chars <= max_chars_per_line
            {
                let mut text = Text::new(s.clone());
                font_info.apply(&mut text);
                texts.push_back((true, text));
                s.clear();
                chars_added = 0;
            }

            if word_chars > max_chars_per_line {
                // If word is too long to fit on a line, then break the word into multiple lines
                for ch in word.chars() {
                    if chars_added == max_chars_per_line {
                        let mut text = Text::new(s.clone());
                        font_info.apply(&mut text);
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
            font_info.apply(&mut text);
            texts.push_back((true, text));
        }

        if let Some((ref mut has_more_texts, _)) = texts.back_mut() {
            *has_more_texts = false;
        }

        texts
    }
}

impl Widget for Chatbox {
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
            return Err(Box::new(UIError::InvalidDimensions{
                reason: format!("Cannot set the size to a width or height of a chatbox, {:?}, to zero", self.id())
            }));
        }

        let old_dims = self.dimensions;
        self.dimensions = new_dims;
        if old_dims.w != new_dims.w {
            // width changed
            self.reflow_messages();
        }
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
                reason: format!("Cannot set the width or height of Chatbox {:?} to zero", self.id())
            }));
        }

        let old_w = self.dimensions.w;
        self.dimensions.w = w;
        self.dimensions.h = h;

        if old_w != w {
            self.reflow_messages();
        }

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // TODO: Add support to scroll through history
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

        let text_entry_rect = Rect::new(
            self.dimensions.x,
            self.dimensions.bottom(),
            self.dimensions.w,
            constants::CHAT_TEXTFIELD_HEIGHT
        );
        let border = graphics::Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(constants::CHATBOX_BORDER_PIXELS),
            text_entry_rect,
            self.color,
        )?;
        graphics::draw(ctx, &border, DrawParam::default())?;

        let mut max_lines = (self.dimensions.h / (self.font_info.char_dimensions.y
                                                    + constants::CHATBOX_LINE_SPACING)) as u32;

        // Draw as many messages as we can fit in the dimensions of the chatbox, newest at the bottom
        let mut i = 0;
        let bottom_left_corner = Point2::new(
            self.dimensions.x,
            self.dimensions.y + self.dimensions.h - self.font_info.char_dimensions.y
        );

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

    fn as_emit_event(&mut self) -> Option<&mut dyn EmitEvent> {
        Some(self)
    }
}

widget_from_id!(Chatbox);
impl_emit_event!(Chatbox, self.handler_data);


pub struct ChatboxPublishHandle {
    msg_sender: Sender<String>,
}

impl ChatboxPublishHandle {
    pub fn add_message(&mut self, msg: String) {
        self.msg_sender.send(msg).unwrap_or_else(|_e| {
            error!("Chatbox has been dropped!");
        });
    }

    pub fn new(msg_sender: Sender<String>) -> Self {
        ChatboxPublishHandle {
            msg_sender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ggez::graphics::Scale;
    use std::collections::vec_deque;

    // Utilities
    fn max_chars_chatbox(max_chars_per_line: usize) -> Chatbox {
        let history_lines = 20;
        let font_info = FontInfo {
            font: (), //dummy font because we can't create a real Font without ggez
            scale: Scale::uniform(1.0), // I don't think this matters
            char_dimensions: Vector2::<f32>::new(5.0, 5.0),  // any positive values will do
        };
        let height = 123.0; // doesn't matter
        // The following must be the reverse of the `max_chars_per_line` calculation in
        // `reflow_message`, plus 0.01 padding.
        let width = font_info.char_dimensions.x * (max_chars_per_line as f32) + 0.01;
        let mut cb = Chatbox::new(font_info, history_lines);
        let _result = cb.set_rect(Rect::new(0.0, 0.0, width, height));
        cb
    }

    // Read the next item from the iterator and compare it. Trailing whitespace is removed before
    // comparison.
    fn compare_next(text_iter: &mut vec_deque::Iter<(bool, Text)>, expected: &str) {
        assert_eq!(text_iter.next().unwrap().1.contents().trim_end(), expected.trim_end());
    }

    // Tests
    #[test]
    fn chatbox_reflow_all_fit() {
        let mut cb = max_chars_chatbox(20);
        cb.add_message("what a great game".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "what a great game");
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_just_barely_fit() {
        let s = "what a great game".to_owned();
        let mut cb = max_chars_chatbox(s.len()); // note: won't work if any multi-byte characters
        cb.add_message(s.clone());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, &s);
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_short_words_no_fit_bound_at_end_of_word() {
        let mut cb = max_chars_chatbox(12);
        cb.add_message("what a great game".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "what a great");
        compare_next(&mut text_iter, "game");
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_short_words_no_fit_bound_in_middle_of_word() {
        let mut cb = max_chars_chatbox(15);
        cb.add_message("what a great game".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "what a great");
        compare_next(&mut text_iter, "game");
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_short_words_no_fit_bound_at_start_of_word() {
        let mut cb = max_chars_chatbox(13);
        cb.add_message("what a great game".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "what a great");
        compare_next(&mut text_iter, "game");
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_short_words_plus_long_word_on_same_line() {
        let mut cb = max_chars_chatbox(9);
        cb.add_message("what an entertaining game".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "what an e");
        compare_next(&mut text_iter, "ntertaini");
        compare_next(&mut text_iter, "ng game");
        assert!(text_iter.next().is_none());
    }

    #[test]
    fn chatbox_reflow_long_word_at_start() {
        let mut cb = max_chars_chatbox(10);
        cb.add_message("entertaining".to_owned());
        cb.reflow_messages();
        let mut text_iter = cb.wrapped.iter();
        compare_next(&mut text_iter, "entertaini");
        compare_next(&mut text_iter, "ng");
        assert!(text_iter.next().is_none());
    }
}
