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
use ggez::graphics::{self, Color, Rect, Font, Text, TextFragment, DrawParam, Drawable};
use ggez::nalgebra::{Point2, Vector2};

use std::rc::Rc;

use super::{
    widget::Widget,
    WidgetID
};

use crate::constants::DEFAULT_UI_FONT_SCALE;

pub struct Label {
    pub textfrag: TextFragment,
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
    pub fn new(ctx: &mut Context, font: Rc<Font>, string: String, color: Color, dest: Point2<f32>) -> Self {
        let font: Font = *font;

        let text_fragment = TextFragment::new(string.clone())
            .scale(*DEFAULT_UI_FONT_SCALE)
            .color(color)
            .font(font);

        let text = Text::new(text_fragment.clone());
        // unwrap safe b/c if this fails then the game is fundamentally broken and is not in a usable state
        let mut dimensions = <Text as Drawable>::dimensions(&text, ctx).unwrap();
        dimensions.move_to(dest);

        Label {
            textfrag: text_fragment,
            dimensions: dimensions
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.textfrag.color = Some(color);
    }

    pub fn set_text(&mut self, string: String) {
        self.textfrag.text = string;
    }
}

impl Widget for Label {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> WidgetID {
        WidgetID::InGameLayer1
    }

    /// Get the size of the widget. Widget must be sizable.
    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
    }

    /// Translate the widget from one location to another. Widget must be sizable.
    fn translate(&mut self, point: Vector2<f32>) {
        self.dimensions.translate(point);
    }

    fn draw(&mut self, ctx: &mut Context, _font: &Font) -> GameResult<()> {

        let text = Text::new(self.textfrag.clone());

        // If the text is updated, we need to refresh the dimensions of the virtual rectangle bounding it.
        // unwrap safe b/c if this fails then the game is fundamentally broken and is not in a usable state
        let recalculated = <Text as Drawable>::dimensions(&text, ctx).unwrap();
        if recalculated.w != self.dimensions.w  && recalculated.h != self.dimensions.h {
            self.dimensions.w = recalculated.w;
            self.dimensions.h = recalculated.h;
        }

        graphics::draw(ctx, &text, DrawParam::default().dest(self.dimensions.point()))?;

        Ok(())
    }
}