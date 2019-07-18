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

use ggez::graphics::{self, Rect, Font, Point2, Color, DrawMode, Vector2};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text, color_with_alpha},
    UIAction, WidgetID
};

pub struct Button {
    pub id:     WidgetID,
    pub label: Label,
    pub button_color: Color,
    pub draw_mode: DrawMode,
    pub dimensions: Rect,
    pub hover: bool,
    pub borderless: bool,
    pub action: UIAction
}

impl Button {
    pub fn new(font: &Font, button_text: &'static str, widget_id: WidgetID, action: UIAction) -> Self {
        const OFFSET_X: f32 = 8.0;
        const OFFSET_Y: f32 = 4.0;
        let width = font.get_width(button_text) as f32 + OFFSET_X*2.0;
        let height = font.get_height() as f32 + OFFSET_Y*2.0;
        let dimensions = Rect::new(30.0, 20.0, width, height);
        let offset = Point2::new(dimensions.x + OFFSET_X, OFFSET_Y);

        Button {
            id: widget_id,
            label: Label::new(font, button_text, color_with_alpha(css::WHITE, 0.1), offset),
            button_color: color_with_alpha(css::DARKCYAN, 0.8),
            draw_mode: DrawMode::Fill,
            dimensions: dimensions,
            hover: false,
            borderless: false,
            action: action,
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

impl Widget for Button {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn on_hover(&mut self, point: &Point2) {
        self.hover = within_widget(point, &self.dimensions);
        if self.hover {
            //println!("Hovering over Button, \"{}\"", self.label.text);
        }
    }

    fn on_click(&mut self, _point: &Point2) -> Option<(WidgetID, UIAction)>
    {
        let hover = self.hover;
        self.hover = false;

        if hover {
            println!("Clicked Button, \"{}\"", self.label.text);
            return Some((self.id, self.action));
        }
        None
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {
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

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
    }

    fn translate(&mut self, point: Vector2)
    {
        self.dimensions.translate(point);
    }
}
