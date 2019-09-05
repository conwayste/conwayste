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

use ggez::{Context, GameResult};
use ggez::graphics::{self, Color, Rect, Font, Text, TextFragment, Scale, DrawParam};
use ggez::nalgebra::{Point2, Vector2};

use super::{
    DEFAULT_UI_FONT_SCALE,
    widget::Widget,
    WidgetID
};

pub struct Label {
    pub text: &'static str,
    pub color: Color,
    pub destination: Point2<f32>,
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
    pub fn new(string: &'static str, color: Color, dest: Point2<f32>) -> Self {

        Label {
            text: string,
            color: color,
            destination: dest,
        }
    }

    pub fn set_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn set_text(mut self, string: &'static str) -> Self {
        self.text = string;
        self
    }
}

impl Widget for Label {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> WidgetID {
        WidgetID::InGameLayer1
    }

    /// Get the size of the widget. Widget must be sizable.
    fn size(&self) -> Rect {
        let mut rect = Rect::new(0.0, 0.0, DEFAULT_UI_FONT_SCALE, DEFAULT_UI_FONT_SCALE*(self.text.len() as f32));
        rect.move_to(self.destination);
        rect
    }

    /// Get the size of the widget. Widget must be sizable.
    fn set_size(&mut self, new_dimensions: Rect) {
        ()
    }

    /// Translate the widget from one location to another. Widget must be sizable.
    fn translate(&mut self, point: Vector2<f32>) {
        let point: Point2<f32> = point.into();
        self.destination =  Point2::new(self.destination.x + point.x, self.destination.y + point.y);
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {

        let text = TextFragment::new(self.text).color(self.color).scale(Scale::uniform(DEFAULT_UI_FONT_SCALE)).font(*font);
        let text = Text::new(text);

        graphics::draw(ctx, &text, DrawParam::default().dest(self.destination))?;

        Ok(())
    }
}