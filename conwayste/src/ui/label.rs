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

use ggez::graphics::{Color, Rect, Font, Point2};

pub struct Label {
    pub text: &'static str,
    pub color: Color,
    pub dimensions: Rect,
}

impl Label {
    pub fn new(font: &Font, text: &'static str, color: Color, origin: Point2) -> Self {
        let w = font.get_width(text) as f32;
        let h = font.get_height() as f32;

        Label {
            text: text,
            color: color,
            dimensions: Rect::new(origin.x, origin.y, w, h),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn set_text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }
}
