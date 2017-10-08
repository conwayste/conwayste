/*  Copyright 2017 the Conwayste Developers.
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

use ggez::graphics::{Rect, draw, Font, Text, Point};
use ggez::Context;

pub struct Graphics;

impl Graphics {

    pub fn draw_text(_ctx: &mut Context, font: &Font, text: &str, coords: &Point, adjustment: Option<&Point>) {
        let mut graphics_text = Text::new(_ctx, text, font).unwrap();
        let dst;

        if let Some(offset) = adjustment {
            dst = Rect::new(coords.x() + offset.x(), coords.y() + offset.y(), graphics_text.width(), graphics_text.height());
        }
        else {
            dst = Rect::new(coords.x(), coords.y(), graphics_text.width(), graphics_text.height());
        }
        let _ = draw(_ctx, &mut graphics_text, None, Some(dst));
    }
}
