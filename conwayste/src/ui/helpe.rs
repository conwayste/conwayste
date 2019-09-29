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

use std::rc::Rc;

use ggez::graphics::{self, Font, Rect, Text, TextFragment, DrawParam, Color};
use ggez::nalgebra::Point2;
use ggez::{Context, GameResult};

use crate::constants::DEFAULT_UI_FONT_SCALE;

/// Helper function to draw text onto the screen.
/// Given the string `str`, it will be drawn at the point coordinates specified by `coords`.
/// An offset can be specified by an optional `adjustment` point.
///
/// # Return value
///
/// On success, an `Ok((text_width, text_height))` tuple is returned, indicating the width
/// and height of the text in pixels.
pub fn draw_text(ctx: &mut Context, font: Rc<Font>, color: Color, text: String, coords: &Point2<f32>) -> GameResult<(f32, f32)> {
    let text_fragment = TextFragment::new(text)
        .scale(*DEFAULT_UI_FONT_SCALE)
        .color(color)
        .font(*font);

    let mut graphics_text = Text::new(text_fragment);
    let (text_width, text_height) = (graphics_text.width(ctx), graphics_text.height(ctx));

    graphics::draw(ctx, &mut graphics_text, DrawParam::default().dest(*coords))?; // actually draw the text!
    Ok((text_width as f32, text_height as f32))
}

/// Determines if two rectangles overlap, and if so,
/// will return `Some` rectangle which spans that overlap.
/// This is a clone of the SDL2 intersection API.
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

/// Provides a new `Point2` from the specified point a the specified offset.
pub fn point_offset(p1: Point2<f32>, x: f32, y: f32) -> Point2<f32> {
    Point2::new(p1.x + x, p1.y + y)
}

/// Calculates the center coordinate of the provided rectangle
pub fn center(r: &Rect) -> Point2<f32> {
    Point2::new((r.left() + r.right()) / 2.0, (r.top() + r.bottom()) / 2.0)
}

/// Checks to see if the boundary defined by the provided rectangle contains the specified point
pub fn within_widget(point: &Point2<f32>, bounds: &Rect) -> bool {
    bounds.contains(*point)
}

pub fn color_with_alpha((r, g, b): (u8, u8, u8), alpha: f32) -> Color {
    Color::from( (r, g, b, (alpha * 255.0) as u8) )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_point_offset() {
        let point = Point2::new(1.0, 1.0);
        let point2 = point_offset(point, 5.0, 5.0);
        let point3 = point_offset(point, -5.0, -5.0);

        assert_eq!(point2, Point2::new(6.0, 6.0));
        assert_eq!(point3, Point2::new(-4.0, -4.0));
    }

    #[test]
    fn test_rectangle_intersection_overlap() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(50.0, 50.0, 150.0, 150.0);
        let rect3 = Rect::new(50.0, 50.0, 50.0, 50.0);

        assert_eq!(intersection(rect1, rect2), Some(rect3));
    }

    #[test]
    fn test_rectangle_intersection_no_overlap() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(150.0, 150.0, 150.0, 150.0);

        assert_eq!(intersection(rect1, rect2), None);
    }
}