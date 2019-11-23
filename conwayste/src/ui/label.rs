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
use ggez::graphics::{self, Color, Rect, Text, TextFragment, DrawParam, Drawable};
use ggez::nalgebra::{Point2, Vector2};
#[cfg(test)]
use ggez::graphics::Font;

use super::{
    common::FontInfo,
    widget::Widget,
    UIError, UIResult,
    WidgetID
};

pub struct Label {
    pub id: WidgetID,
    pub textfrag: TextFragment,
    pub dimensions: Rect,
}

/// A graphical widget representation of text
impl Label {
    /// Creates a Label widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `widget_id` - Unique widget identifier
    /// * `font` - font to be used when drawing the text
    /// * `string` - Label text
    /// * `color` - Text color
    /// * `dest` - Destination point
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ui::{self, Label, common};
    /// use chromatica::css;
    ///
    /// let font = Font::default();
    /// let font_info = FontInfo::new(font, Some(20.0));
    /// let label = Label::new(
    ///     ctx,
    ///     ui::TestLabel,
    ///     font_info,
    ///     "TestButton",
    ///     Color::from(css::DARKCYAN),
    ///     Color::from(css::WHITE)
    /// );
    ///
    /// label.draw(ctx);
    /// ```
    ///
    //TODO: remove ctx; no need to make TextFragments if we know the width of characters (font_info)
    pub fn new(
        ctx: &mut Context,
        widget_id: WidgetID,
        font_info: FontInfo,
        string: String,
        color: Color,
        dest: Point2<f32>
    ) -> Self {
        let text_fragment;
        #[cfg(not(test))]
        {
            text_fragment = TextFragment::new(string.clone())
                .scale(font_info.scale)
                .color(color)
                .font(font_info.font);
        }
        #[cfg(test)]
        {
            text_fragment = TextFragment::new(string.clone())
                .scale(font_info.scale)
                .color(color)
                .font(Font::default());
        }

        let text = Text::new(text_fragment.clone());
        // unwrap safe b/c if this fails then the game is fundamentally broken and is not in a usable state
        let mut dimensions = <Text as Drawable>::dimensions(&text, ctx).unwrap();
        dimensions.move_to(dest);

        Label {
            id: widget_id,
            textfrag: text_fragment,
            dimensions: dimensions
        }
    }
}

impl Widget for Label {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> WidgetID {
        self.id
    }

    /// Get the size of the widget. Widget must be sizable.
    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of Label {:?} to zero", self.id())
            }));
        }

        self.dimensions = new_dims;
        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.dimensions.point().into()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.dimensions.x = x;
        self.dimensions.y = y;
    }

    fn size(&self) -> (f32, f32) {
        (self.dimensions.w, self.dimensions.h)
    }

    fn set_size(&mut self, w: f32, h: f32) -> UIResult<()> {
        if w == 0.0 || h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of Label {:?} to zero", self.id())
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }

    /// Translate the widget from one location to another. Widget must be sizable.
    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        let text = Text::new(self.textfrag.clone());

        // If the text is updated, we need to refresh the dimensions of the virtual rectangle bounding it.
        // unwrap safe b/c if this fails then the game is fundamentally broken and is not in a usable state
        let recalculated = <Text as Drawable>::dimensions(&text, ctx).unwrap();
        if recalculated.w != self.dimensions.w  || recalculated.h != self.dimensions.h {
            self.dimensions.w = recalculated.w;
            self.dimensions.h = recalculated.h;
        }

        graphics::draw(ctx, &text, DrawParam::default().dest(self.dimensions.point()))?;

        Ok(())
    }
}

widget_from_id!(Label);
