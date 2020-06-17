/*  Copyright 2019-2020 the Conwayste Developers.
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

use std::fmt;

use ggez::{Context, GameResult};
use ggez::graphics::{self, Color, Rect, Text, TextFragment, DrawParam, Drawable};
use ggez::nalgebra::{Point2, Vector2};
#[cfg(test)]
use ggez::graphics::Font;

use id_tree::NodeId;

use super::{
    common::FontInfo,
    widget::Widget,
    UIError, UIResult,
    context::{EmitEvent, HandlerData},
};

pub struct Label {
    id: Option<NodeId>,
    font_info: FontInfo,
    color: Color,
    z_index: usize,
    pub textfrag: TextFragment,
    pub dimensions: Rect,
    handler_data: HandlerData,
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Label {{ id: {:?}, z-index: {}, Dimensions: {:?} }}", self.id, self.z_index, self.dimensions)
    }
}

/// A graphical widget representation of text
impl Label {
    /// Creates a Label widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
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
    ///     font_info,
    ///     "TestButton",
    ///     Color::from(css::DARKCYAN),
    ///     Color::from(css::WHITE)
    /// );
    ///
    /// label.draw(ctx);
    /// ```
    ///
    pub fn new(
        ctx: &mut Context,
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
            id: None,
            font_info,
            color,
            z_index: std::usize::MAX,
            textfrag: text_fragment,
            dimensions,
            handler_data: HandlerData::new(),
        }
    }

    /// Sets the text for this label. Note that the dimensions are changed by this.
    pub fn set_text(&mut self, ctx: &mut Context, text: String) {
        let dest = self.dimensions.point();
        let text_fragment;
        #[cfg(not(test))]
        {
            text_fragment = TextFragment::new(text)
                .scale(self.font_info.scale)
                .color(self.color)
                .font(self.font_info.font);
        }
        #[cfg(test)]
        {
            text_fragment = TextFragment::new(text)
                .scale(self.font_info.scale)
                .color(self.color)
                .font(Font::default());
        }

        let text = Text::new(text_fragment.clone());
        // unwrap safe b/c if this fails then the game is fundamentally broken and is not in a usable state
        let mut dimensions = <Text as Drawable>::dimensions(&text, ctx).unwrap();
        dimensions.move_to(dest);
        self.dimensions = dimensions;
        self.textfrag = text_fragment;
    }

    /// Gets the text set for this label.
    #[allow(unused)]
    pub fn text(&self) -> &str {
        &self.textfrag.text
    }
}

impl Widget for Label {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> Option<&NodeId> {
        self.id.as_ref()
    }

    fn set_id(&mut self, new_id: NodeId) {
        self.id = Some(new_id);
    }

    fn z_index(&self) -> usize {
        self.z_index
    }

    fn set_z_index(&mut self, new_z_index: usize) {
        self.z_index = new_z_index;
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

    fn as_emit_event(&mut self) -> Option<&mut dyn EmitEvent> {
        Some(self)
    }
}

widget_from_id!(Label);
impl_emit_event!(Label, self.handler_data);
