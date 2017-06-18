extern crate ggez;

use ggez::graphics::{Rect, draw, Font, Text, Point};
// use ggez::graphics::{Rect, Color};
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
