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

use ggez::graphics::{self, Rect, Font, Text, Point2, Color, DrawMode};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text}
    };

pub struct Button<T> {
    pub label: Label,
    pub button_color: Color,
    pub draw_mode: DrawMode,
    pub dimensions: Rect,
    pub hover: bool,
    pub borderless: bool,
    pub click: Box<dyn FnMut(&mut T)>
}

impl<T> Button<T> {
    pub fn new(font: &Font, button_text: &'static str, action: Box<dyn FnMut(&mut T)>) -> Self {
        const OFFSET_X: f32 = 8.0;
        const OFFSET_Y: f32 = 4.0;
        let width = font.get_width(button_text) as f32 + OFFSET_X*2.0;
        let height = font.get_height() as f32 + OFFSET_Y*2.0;
        let dimensions = Rect::new(30.0, 20.0, width, height);
        let offset = Point2::new(dimensions.x + OFFSET_X, OFFSET_Y);

        Button {
            label: Label::new(font, button_text, Color::from(css::WHITE), offset),
            button_color: Color::from(css::DARKCYAN),
            draw_mode: DrawMode::Fill,
            dimensions: dimensions,
            hover: false,
            borderless: false,
            click: action,
        }
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label = self.label.set_color(color);
        self
    }

    pub fn button_color(mut self, color: Color) -> Self {
        self.button_color = color;
        self
    }
}

impl<T> Widget<T> for Button<T> {
    fn on_hover(&mut self, point: &Point2) {
        self.hover = within_widget(point, &self.dimensions);
        //println!("Hovering over Button, \"{}\"", self.label);
    }

    fn on_click(&mut self, point: &Point2, t: &mut T)
    {
        if within_widget(point, &self.dimensions) {
            println!("Clicked Button, \"{}\"", self.label.text);
            (self.click)(t)
        }
    }

    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let old_color = graphics::get_color(ctx);
        graphics::set_color(ctx, self.button_color)?;

        let draw_mode = if self.hover {
            DrawMode::Fill
        } else {
            DrawMode::Line(2.0)
        };

        graphics::rectangle(ctx, draw_mode, self.dimensions)?;
        draw_text(ctx, font, self.label.color, &self.label.text, &self.dimensions.point(), None)?;

        graphics::set_color(ctx, old_color)
    }
}
