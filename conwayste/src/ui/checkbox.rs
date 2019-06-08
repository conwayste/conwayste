
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

use ggez::graphics::{self, Rect, Font, Point2, Color, DrawMode};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text}
    };

#[derive(PartialEq)]
pub enum ToggleState {
    Disabled,
    Enabled
}

pub struct Checkbox<T> {
    pub label: Label,
    pub state: ToggleState,
    pub dimensions: Rect,
    pub hover: bool,
    pub click: Box<dyn FnMut(&mut T)>
}

impl<T> Checkbox<T> {
    pub fn new(font: &Font, text: &'static str, dimensions: Rect, action: Box<dyn FnMut(&mut T)>) -> Self {
        const LABEL_OFFSET_X: f32 = 20.0;
        const LABEL_OFFSET_Y: f32 = -25.0;

        let label_origin = Point2::new(dimensions.x + dimensions.w + LABEL_OFFSET_X, dimensions.y + dimensions.h + LABEL_OFFSET_Y);

        Checkbox {
            label: Label::new(font, text, Color::from(css::WHITE), label_origin),
            state: ToggleState::Disabled,
            dimensions: dimensions,
            hover: false,
            click: action
        }
    }

    pub fn toggle(&mut self) {
        if self.state == ToggleState::Disabled {
            self.state = ToggleState::Enabled
        } else {
            self.state = ToggleState::Disabled
        }
    }

}


impl<T> Widget<T> for Checkbox<T> {
    fn on_hover(&mut self, point: &Point2) {
        self.hover = within_widget(point, &self.dimensions) || within_widget(point, &self.label.dimensions);
        //if self.hover {
        //    println!("Hovering over Checkbox, \"{:?}\"", self.label.dimensions);
        //}
    }

    fn on_click(&mut self, point: &Point2, t: &mut T)
    {
        if within_widget(point, &self.dimensions) || within_widget(point, &self.label.dimensions) {
            println!("Clicked Checkbox, \"{}\"", self.label.text);
            self.toggle();
            (self.click)(t)
        }
    }

    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let old_color = graphics::get_color(ctx);
        graphics::set_color(ctx, self.label.color)?;

        let draw_mode = if self.state == ToggleState::Enabled {
            DrawMode::Fill
        } else {
            DrawMode::Line(1.0)
        };

        if self.hover {
            // Add in a violet border/fill while hovered. Color checkbox differently to indicate  hovered state.
            let border_rect = Rect::new(self.dimensions.x-1.0, self.dimensions.y-1.0, self.dimensions.w + 4.0, self.dimensions.h + 4.0);
            graphics::set_color(ctx, Color::from(css::VIOLET))?;
            graphics::rectangle(ctx, DrawMode::Line(2.0), border_rect)?;
        }

        graphics::rectangle(ctx, draw_mode, self.dimensions)?;
        draw_text(ctx, font, self.label.color, &self.label.text, &self.label.dimensions.point(), None)?;

        graphics::set_color(ctx, old_color)
    }
}
