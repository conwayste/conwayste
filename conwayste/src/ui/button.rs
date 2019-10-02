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

use chromatica::css;

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, color_with_alpha, center},
    UIAction, WidgetID
};

pub struct Button {
    pub id:     WidgetID,
    pub label: Label,
    pub button_color: Color,
    pub draw_mode: DrawMode,
    pub dimensions: Rect,
    pub hover: bool,
    pub borderless: bool,
    pub action: UIAction
}

const BUTTON_LABEL_PADDING_W: f32 = 16.0;   // in pixels
const BUTTON_LABEL_PADDING_H: f32 = 16.0;   // in pixels

/// A named widget that can be clicked to result in an occuring action.
impl Button {

    /// Creates a Button widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `font` - font to be used when drawing the text
    /// * `button_text` - Text to be displayed
    /// * `widget_id` - Unique widget identifier
    /// * `action` - Unique action identifer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::Button;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Font::default();
    ///     let b = Button::new(ctx, ui::TestButton1,
    ///             UIAction::PrintHelloWorld,
    ///             ui::TestButton1Label,
    ///             font,
    ///             "TestButton");
    ///
    ///     b.draw(ctx)?;
    /// }
    /// ```
    ///
    pub fn new(ctx: &mut Context, widget_id: WidgetID, action: UIAction, label_id: WidgetID,
        font: Font, button_text: String) -> Self
    {
        let label_position = Point2::new(0.0, 0.0); // label positioning defined an offset to button origin after centering
        let label = Label::new(ctx, label_id, font, button_text, color_with_alpha(css::WHITE, 0.1), label_position);
        let label_dims = label.size();

        let dimensions = Rect::new(30.0, 20.0, label_dims.w + BUTTON_LABEL_PADDING_W, label_dims.h + BUTTON_LABEL_PADDING_H);

        let mut b = Button {
            id: widget_id,
            label: label,
            button_color: color_with_alpha(css::DARKCYAN, 0.8),
            draw_mode: DrawMode::fill(),
            dimensions: dimensions,
            hover: false,
            borderless: false,
            action: action,
        };
        b.center_label_text();
        b
    }

    fn center_label_text(&mut self) {
        let text_dims = self.label.size();
        let tmp_label_rect = Rect::new(self.dimensions.x, self.dimensions.y, text_dims.w, text_dims.h);
        let label_center_point = center(&tmp_label_rect);
        let button_center = center(&self.dimensions);

        self.label.set_size(Rect::new(self.dimensions.x + (button_center.x - label_center_point.x),
            self.dimensions.y + (button_center.y - label_center_point.y),
            text_dims.w,
            text_dims.h));
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        // if self.hover {
        //     debug!("Hovering over Button, \"{}\" {:?}", self.label.text, point);
        // }
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)>
    {
        let hover = self.hover;
        self.hover = false;

        if hover {
            debug!("Clicked Button, '{:?}'", self.label.textfrag);
            return Some((self.id, self.action));
        }
        None
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let draw_mode = if self.hover {
            DrawMode::fill()
        } else {
            DrawMode::stroke(2.0)
        };

        let button = graphics::Mesh::new_rectangle(ctx, draw_mode, self.dimensions, self.button_color)?;
        graphics::draw(ctx, &button, DrawParam::default())?;

        self.label.draw(ctx)?;

        Ok(())
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        if new_dims.w < self.label.dimensions.w + BUTTON_LABEL_PADDING_W
        || new_dims.h < self.label.dimensions.h + BUTTON_LABEL_PADDING_H {
            // PR_GATE add error handling
            // cannot set the size of a button to anything smaller than the self-containing text plus some margin
            return;
        }
        self.dimensions = new_dims;
        self.center_label_text();
    }

    fn translate(&mut self, dest: Vector2<f32>)
    {
        self.dimensions.translate(dest);
        self.label.translate(dest);
    }
}
