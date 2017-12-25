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

    pub fn intersection(A: Rect, B: Rect) -> Option<Rect> {

        fn EmptyRect(R: Rect) -> bool {
            R.w <= 0.0 || R.h <= 0.0
        }

        let mut result = Rect::zero();

        if EmptyRect(A) || EmptyRect(B) {
            return None;
        }

        let mut A_min = A.x;
        let mut A_max = A_min + A.w;
        let mut B_min = B.x;
        let mut B_max = B_min + B.w;

        /* Horizontal Intersection*/
        if B_min > A_min {
            A_min = B_min;
        }
        result.x = A_min;

        if B_max < A_max {
            A_max = B_max;
        }
        result.w = A_max - A_min;

        /* Veritcal Intersection */
        A_min = A.y;
        A_max = A_min + A.h;
        B_min = B.y;
        B_max = B_min + B.h;

        if B_min > A_min {
            A_min = B_min;
        }
        result.y = A_min;

        if B_max < A_max {
            A_max = B_max;
        }
        result.h = A_max - A_min;

        if EmptyRect(result) {
            return None;
        } else {
            return Some(result);
        }
    }

    pub fn point_offset(p1: Point2, x: f32, y: f32) -> Point2 {
        Point2::new(p1.x + x, p1.y + y)
    }
}
