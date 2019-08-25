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

use ggez::Context;
use ggez::graphics::{Color, Rect, Font, Text, Scale};
use ggez::nalgebra::Point2;

pub struct Label {
    pub text: &'static str,
    pub color: Color,
    pub dimensions: Rect,
}

/// A graphical widget representation of text
impl Label {
    /// Creates a Label widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `font` - font to be used when drawing the text
    /// * `text` - Label text
    /// * `color` - Text color
    /// * `widget_id` - Unique widget identifier
    /// * `action` - Unique action identifer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ui::Button;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Font::default();
    ///     let b = Button::new(ctx, font, "TestButton", WidgetID::TestButton1, UIAction::PrintHelloWorld)
    ///         .label_color(Color::from(css::DARKCYAN))
    ///         .button_color(Color::from(css::WHITE));
    ///
    ///     b.draw(ctx, font)?;
    /// }
    /// ```
    ///
    pub fn new(ctx: &mut Context, font: &Font, string: &'static str, color: Color, origin: Point2<f32>) -> Self {
        // TODO pass in as a parameter the scale
        let mut text = Text::new(string);
        text.set_font(*font, Scale::uniform(10.0));
        let w = text.width(ctx) as f32;
        let h = text.height(ctx) as f32;

        Label {
            text: string,
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
