
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

extern crate ggez;
extern crate chromatica;

use ggez::graphics::{self, Rect, Font, Text, Point2, Color, DrawMode};
use ggez::{Context, GameResult};
use chromatica::css;
use crate::utils;

pub trait Widget<T> {
    fn on_hover(&mut self, point: &Point2);
    fn on_click(&mut self, point: &Point2, t: &mut T);
    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()>;
}

pub struct Button<T> {
    label: &'static str,
    label_color: Color,
    button_color: Color,
    draw_mode: DrawMode,
    pub dimensions: Rect,
    hover: bool,
    click: Box<dyn FnMut(&mut T)>
}

impl<T> Button<T> {
    pub fn new(font: &Font, label: &'static str, action: Box<dyn FnMut(&mut T)>) -> Self {
        let offset = Point2::new(8.0, 4.0);
        let width = font.get_width(label) as f32 + offset.x*2.0;
        let height = font.get_height() as f32 + offset.y*2.0;

        Button {
            label: label,
            label_color: Color::from(css::WHITE),
            button_color: Color::from(css::DARKCYAN),
            draw_mode: DrawMode::Fill,
            dimensions: Rect::new(30.0, 20.0, width, height),
            hover: false,
            click: action,
        }
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
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
            println!("Clicked Button, \"{}\"", self.label);
            (self.click)(t)
        }
    }

    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let offset = Point2::new(8.0, 4.0);
        graphics::set_color(ctx, self.button_color)?;
        let draw_mode = if self.hover {
            DrawMode::Fill
        } else {
            DrawMode::Line(2.0)
        };
        graphics::rectangle(ctx, draw_mode, self.dimensions)?;
        utils::Graphics::draw_text(ctx, font, self.label_color, &self.label, &self.dimensions.point(), Some(&offset))
    }
}

fn within_widget(point: &Point2, bounds: &Rect) -> bool {
    bounds.contains(*point)
}

fn center(r: &Rect) -> Point2 {
    Point2::new((r.left() + r.right()) / 2.0, (r.top() + r.bottom()) / 2.0)
}