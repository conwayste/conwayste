
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

use std::fmt;

use ggez::graphics::{self, Rect, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    common::{within_widget, FontInfo},
    UIAction,
    UIError, UIResult,
    WidgetID,
};

use crate::constants::colors::*;

pub struct Checkbox {
    id: WidgetID,
    z_index: usize,
    pub label: Label,
    pub enabled: bool,
    pub dimensions: Rect,
    pub hover: bool,
    pub action: UIAction
}

impl fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Checkbox {{ id: {:?}, z-index: {}, Dimensions: {:?}, Action: {:?}, Checked: {} }}",
            self.id, self.z_index, self.dimensions, self.action, self.enabled)
    }
}

const LABEL_OFFSET_X: f32 = 30.0;
const LABEL_OFFSET_Y: f32 = -5.0;

/// A standard checkbox widget that can be toggled between enabled or disabled
impl Checkbox {
    /// Creates a Checkbox widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `widget_id` - Unique widget identifier
    /// * `enabled` - initial to checked or unchecked
    /// * `font_info` - font descriptor to be used when drawing the text
    /// * `text` - Label text
    /// * `dimensions` - Size of checkbox (currently a hollor or filled rectangle)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::{self, Checkbox, common};
    ///
    /// let font = Font::default();
    /// let font_info = common::FontInfo::new(ctx, font, Some(20.0));
    /// let checkbox = Checkbox::new(
    ///     ctx,
    ///     ui::TestCheckbox,
    ///     false,
    ///     font_info,
    ///     "Toggle Me",
    ///     Rect::new(10.0, 210.0, 20.0, 20.0)
    /// );
    /// checkbox.draw(ctx);
    /// ```
    ///
    pub fn new(
        ctx: &mut Context,
        widget_id: WidgetID,
        enabled: bool,
        font_info: FontInfo,
        text: String,
        dimensions: Rect,
    ) -> Self {
        let label_origin = Point2::new(
            dimensions.x + dimensions.w + LABEL_OFFSET_X,
            dimensions.y + dimensions.h + LABEL_OFFSET_Y
        );

        Checkbox {
            id: widget_id,
            z_index: 0,
            label: Label::new(ctx, widget_id, font_info, text, *CHECKBOX_TEXT_COLOR, label_origin),
            enabled: enabled,
            dimensions: dimensions,
            hover: false,
            action: UIAction::Toggle(enabled)
        }
    }

    /// Toggles the checkbox between enabled or disabled, and returns its new state
    pub fn toggle_checkbox(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }
}


impl Widget for Checkbox {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn z_index(&self) -> usize {
        self.z_index
    }

    fn set_z_index(&mut self, new_z_index: usize) {
        self.z_index = new_z_index;
    }

    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the size to a width or height of Checkbox {:?} to zero", self.id())
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
                reason: format!("Cannot set the width or height of Checkbox {:?} to zero", self.id())
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }


    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
        self.label.translate(dest);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        let label_dimensions = self.label.rect();
        self.hover = within_widget(point, &self.dimensions) || within_widget(point, &label_dimensions);
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)>
    {
        // TODO: Check child label for an on_click event once it's refactored out
        return Some(( self.id, UIAction::Toggle(self.toggle_checkbox()) ));
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        if self.hover {
            // Add in a violet border/fill while hovered. Color checkbox differently to indicate  hovered state.
            let border_rect = Rect::new(
                self.dimensions.x-1.0,
                self.dimensions.y-1.0,
                self.dimensions.w + 4.0,
                self.dimensions.h + 4.0
            );

            let hovered_border = graphics::Mesh::new_rectangle(
                ctx,
                DrawMode::stroke(2.0),
                border_rect,
                *CHECKBOX_BORDER_ON_HOVER_COLOR
            )?;

            graphics::draw(ctx, &hovered_border, DrawParam::default())?;
        }

        let draw_mode = if self.enabled {
            DrawMode::fill()
        } else {
            DrawMode::stroke(2.0)
        };

        let border = graphics::Mesh::new_rectangle(
            ctx,
            draw_mode,
            self.dimensions,
            *CHECKBOX_TOGGLED_FILL_COLOR
        )?;
        graphics::draw(ctx, &border, DrawParam::default())?;

        let label_border = graphics::Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            self.dimensions,
            *CHECKBOX_TOGGLED_FILL_COLOR
        )?;
        graphics::draw(ctx, &label_border, DrawParam::default())?;

        self.label.draw(ctx)?;

        Ok(())
    }
}

widget_from_id!(Checkbox);
