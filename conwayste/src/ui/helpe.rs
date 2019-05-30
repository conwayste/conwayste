
use ggez::graphics::{self, Font, Point2, Rect, Text, Color};
use ggez::{Context, GameResult};

/// Helper function to draw text onto the screen.
/// Given the string `str`, it will be drawn at the point coordinates specified by `coords`.
/// An offset can be specified by an optional `adjustment` point.
pub fn draw_text(_ctx: &mut Context, font: &Font, color: Color, text: &str, coords: &Point2, adjustment: Option<&Point2>) -> GameResult<()> {
    let mut graphics_text = Text::new(_ctx, text, font)?;
    let dst;

    if let Some(offset) = adjustment {
        dst = Point2::new(coords.x + offset.x, coords.y + offset.y);
    }
    else {
        dst = Point2::new(coords.x, coords.y);
    }
    // We store the color being used to simplify code and aid in debuggability, since our
    // drawing code is quite complex now. We don't the caller of this `draw_text` method to
    // care what color we use to draw the text, or to clean up after calling it.
    let previous_color = graphics::get_color(_ctx);      // store previous color
    graphics::set_color(_ctx, color)?;                   // text foreground
    graphics::draw(_ctx, &mut graphics_text, dst, 0.0)?; // actually draw the text!
    graphics::set_color(_ctx, previous_color)?;          // restore previous color
    Ok(())
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
pub fn point_offset(p1: Point2, x: f32, y: f32) -> Point2 {
    Point2::new(p1.x + x, p1.y + y)
}

/// Calculates the center coordinate of the provided rectangle
pub fn center(r: &Rect) -> Point2 {
    Point2::new((r.left() + r.right()) / 2.0, (r.top() + r.bottom()) / 2.0)
}

/// Checks to see if the boundary defined by the provided rectangle contains the specified point
pub fn within_widget(point: &Point2, bounds: &Rect) -> bool {
    bounds.contains(*point)
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
