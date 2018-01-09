/*  Copyright 2017-2018 the Conwayste Developers.
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

    pub fn intersection(a: Rect, b: Rect) -> Option<Rect> {

        fn empty_rect(r: Rect) -> bool {
            r.w <= 0.0 || r.h <= 0.0
        }

        let mut result = Rect::zero();

        if empty_rect(a) || empty_rect(b) {
            return None;
        }

        let mut a_min = a.x;
        let mut a_max = a_min + a.w;
        let mut b_min = b.x;
        let mut b_max = b_min + b.w;

        /* horizontal intersection*/
        if b_min > a_min {
            a_min = b_min;
        }
        result.x = a_min;

        if b_max < a_max {
            a_max = b_max;
        }
        result.w = a_max - a_min;

        /* vertical intersection */
        a_min = a.y;
        a_max = a_min + a.h;
        b_min = b.y;
        b_max = b_min + b.h;

        if b_min > a_min {
            a_min = b_min;
        }
        result.y = a_min;

        if b_max < a_max {
            a_max = b_max;
        }
        result.h = a_max - a_min;

        if empty_rect(result) {
            return None;
        } else {
            return Some(result);
        }
    }

    pub fn point_offset(p1: Point2, x: f32, y: f32) -> Point2 {
        Point2::new(p1.x + x, p1.y + y)
    }
}
