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

use ggez::graphics::{Rect, draw, Font, Text, Point2};
use ggez::Context;

pub struct Graphics;

impl Graphics {

    pub fn draw_text(_ctx: &mut Context, font: &Font, text: &str, coords: &Point2, adjustment: Option<&Point2>) {
        let mut graphics_text = Text::new(_ctx, text, font).unwrap();
        let dst;

        if let Some(offset) = adjustment {
            dst = Point2::new(coords.x + offset.x, coords.y + offset.y);
        }
        else {
            dst = Point2::new(coords.x, coords.y);
        }
        let _ = draw(_ctx, &mut graphics_text, dst, 0.0);
    }

    pub fn intersection(r1: Rect, r2: Rect) -> Option<Rect> {
        let xmin = f32::min(r1.x, r2.x);
        let xmax1 = r1.x + r1.w;
        let xmax2 = r2.x + r2.w;
        let xmax = f32::max(xmax1, xmax2);

        if (xmax > xmin) {
            let ymin = f32::min(r1.y, r2.y);
            let ymax1 = r1.y + r1.h;
            let ymax2 = r2.y + r2.h;
            let ymax = f32::max(ymax1, ymax2);

            if (ymax > ymin) {
                return Some(
                    Rect {
                        x: xmin,
                        y: ymin,
                        w: xmax - xmin,
                        h: ymax - ymin
                    }
                );
            }
        }

        None
    }

    pub fn point_offset(p1: Point2, x: f32, y: f32) -> Point2 {
        Point2::new(p1.x + x, p1.y + y)
    }
}
