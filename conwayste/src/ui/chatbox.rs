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

use std::collections::VecDeque;

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam, Text, BlendMode, FilterMode, TextFragment};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    helpe::within_widget,
    textfield::TextField,
    UIAction, WidgetID
};

const CHAT_DISPLAY_LIMIT: f32 = 10.0;

pub struct Chatbox {
    pub id: WidgetID,
    pub history_len: usize,
    pub color: Color,
    pub messages: VecDeque<Text>,
    pub dimensions: Rect,
    pub hover: bool,
    pub action: UIAction,
}

impl Chatbox {
    pub fn new(widget_id: WidgetID, len: usize) -> Self {
        let rect = Rect::new(30.0, 600.0, 300.0, 15.0*CHAT_DISPLAY_LIMIT);
        Chatbox {
            id: widget_id,
            history_len: len,
            color: Color::from(css::VIOLET),
            messages: VecDeque::with_capacity(len),
            dimensions: rect,
            hover: false,
            action: UIAction::EnterText,
        }
    }

    pub fn add_message(&mut self, ctx: &mut Context, font: &Font, msg: String) -> GameResult<()> {
        if self.messages.len() + 1 > self.history_len {
            self.messages.pop_front();
        }
        // FIXME ggez0.5
        let text = Text::new(msg);
        self.messages.push_back(text);
        Ok(())
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
    }

    fn translate(&mut self, point: Vector2<f32>)
    {
        self.dimensions.translate(point);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        //if self.hover {
        //    println!("Hovering over Chatbox, \"{:?}\"", self.label.dimensions);
        //}
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)>
    {
        let hover = self.hover;
        self.hover = false;

        if hover {
            println!("Clicked within Chatbox");
            return Some((self.id, self.action));
        }

        None
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {

        if self.hover {
            // Add in a teal border while hovered. Color checkbox differently to indicate  hovered state.
            let border_rect = Rect::new(self.dimensions.x-1.0, self.dimensions.y-1.0, self.dimensions.w + 4.0, self.dimensions.h + 4.0);
            let hovered_border = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(2.0), border_rect, Color::from(css::TEAL))?;
            graphics::draw(ctx, &hovered_border, DrawParam::default())?;
        }

        let border = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(4.0), self.dimensions, self.color)?;
        graphics::draw(ctx, &border, DrawParam::default())?;

        // TODO need to do width wrapping check
        for (i, msg) in self.messages.iter().enumerate() {
            let origin = self.dimensions.point();
            let point = Point2::new(origin.x + 5.0, origin.y + i as f32*30.0);

            graphics::queue_text(ctx, &msg, point, Some(Color::from(css::RED)));
        }

        graphics::draw_queued_text(ctx, DrawParam::default().dest(self.dimensions.point()), None, FilterMode::Linear)?;

        Ok(())
    }
}
