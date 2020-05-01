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
use std::error::Error;

use chromatica::css;

use ggez::graphics::{self, Rect, Color, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use id_tree::NodeId;

use super::{
    context,
    label::Label,
    widget::Widget,
    common::{within_widget, color_with_alpha, center, FontInfo},
    UIAction,
    UIError, UIResult,
    context::{
        EmitEvent,
        UIContext,
        EventType,
        Event,
        Handled,
    },
};

pub struct Button {
    id: Option<NodeId>,
    z_index: usize,
    pub label: Label,
    pub button_color: Color,
    pub draw_mode: DrawMode,
    pub dimensions: Rect,
    pub hover: bool, // is mouse hovering over this?
    pub focused: bool, // has keyboard focus?
    pub borderless: bool,
    pub action: UIAction,
    pub handler_data: context::HandlerData, // required for impl_emit_event!
    // option solely so that we can not mut borrow self twice at once
}

impl fmt::Debug for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Button {{ id: {:?}, z-index: {}, Dimensions: {:?}, Action: {:?}}}",
            self.id, self.z_index, self.dimensions, self.action)
    }
}

const BUTTON_LABEL_PADDING_W: f32 = 16.0;   // in pixels
const BUTTON_LABEL_PADDING_H: f32 = 16.0;   // in pixels

/// A named widget that can be clicked to result in an occuring action.
impl Button {

    /// Creates a Button widget. The button's dimensions will automatically be sized to the provided
    /// text.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `action` - Unique action identifer
    /// * `font_info` - font descriptor to be used when drawing the text
    /// * `button_text` - Text to be displayed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ggez::graphics::Font;
    /// use ui::{self, Button, common};
    ///
    /// let font = Font::Default;
    /// let font_info = common::FontInfo::new(ctx, font, Some(20.0));
    /// let b = Button::new(
    ///     ctx,
    ///     UIAction::PrintHelloWorld,
    ///     font_info,
    ///     "TestButton"
    /// );
    ///
    /// b.draw(ctx);
    /// ```
    ///
    pub fn new(
        ctx: &mut Context,
        action: UIAction,
        font_info: FontInfo,
        button_text: String,
    ) -> Self {
        // label positioning defined an offset to button origin after centering
        let label_position = Point2::new(0.0, 0.0);
        let label = Label::new(
            ctx,
            font_info,
            button_text,
            color_with_alpha(css::WHITE, 0.1),
            label_position
        );
        let label_dims = label.rect();

        let dimensions = Rect::new(
            30.0,
            20.0,
            label_dims.w + BUTTON_LABEL_PADDING_W,
            label_dims.h + BUTTON_LABEL_PADDING_H
        );

        let mut b = Button {
            id: None,
            z_index: std::usize::MAX,
            label,
            button_color: color_with_alpha(css::DARKCYAN, 0.8),
            draw_mode: DrawMode::fill(),
            dimensions,
            hover: false,
            focused: false,
            borderless: false,
            action,
            handler_data: context::HandlerData::new(),
        };
        b.center_label_text();

        // setup handler to toggle keyboard focus
        let focus_chg = |obj: &mut dyn EmitEvent, _uictx: &mut UIContext, event: &Event| -> Result<Handled, Box<dyn Error>> {
            let button = obj.downcast_mut::<Button>().unwrap(); // unwrap OK because this will always be Button
            match event.what {
                EventType::GainFocus => button.focused = true,
                EventType::LoseFocus => button.focused = false,
                _ => unimplemented!("this handler is only for gaining/losing focus"),
            };
            Ok(Handled::Handled)
        };
        b.on(EventType::GainFocus, Box::new(focus_chg.clone())).unwrap(); // unwrap OK b/c not being called within handler
        b.on(EventType::LoseFocus, Box::new(focus_chg)).unwrap(); // unwrap OK b/c not being called within handler

        b
    }

    /// Centers the label's text to the dimensions of the button
    fn center_label_text(&mut self) {
        let text_dims = self.label.rect();
        let tmp_label_rect = Rect::new(self.dimensions.x, self.dimensions.y, text_dims.w, text_dims.h);
        let label_center_point = center(&tmp_label_rect);
        let button_center = center(&self.dimensions);

        self.label.set_position(
            self.dimensions.x + (button_center.x - label_center_point.x),
            self.dimensions.y + (button_center.y - label_center_point.y),
        );
    }
}

impl Widget for Button {
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

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<UIAction>
    {
        return Some(self.action);
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let draw_mode = if self.hover || self.focused {
            DrawMode::fill()
        } else {
            DrawMode::stroke(2.0)
        };

        let button = graphics::Mesh::new_rectangle(ctx, draw_mode, self.dimensions, self.button_color)?;
        graphics::draw(ctx, &button, DrawParam::default())?;

        self.label.draw(ctx)?;

        Ok(())
    }

    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of a Button {:?} to zero", self.id())
            }));
        }

        if new_dims.w < self.label.dimensions.w + BUTTON_LABEL_PADDING_W
        || new_dims.h < self.label.dimensions.h + BUTTON_LABEL_PADDING_H {
            return Err(Box::new(UIError::InvalidDimensions{
                reason: format!("Cannot set the Button's size smaller than the space taken by the
                    button's text: {:?}", self.id())
            }));
        }

        self.dimensions = new_dims;
        self.center_label_text();
        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.dimensions.point().into()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.dimensions.x = x;
        self.dimensions.y = y;

        self.center_label_text();
    }

    fn size(&self) -> (f32, f32) {
        (self.dimensions.w, self.dimensions.h)
    }

    fn set_size(&mut self, w: f32, h: f32) -> UIResult<()> {
        if w == 0.0 || h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of Button {:?} to zero", self.id())
            }));
        }

        if w < self.label.dimensions.w + BUTTON_LABEL_PADDING_W
        || h < self.label.dimensions.h + BUTTON_LABEL_PADDING_H {
            return Err(Box::new(UIError::InvalidDimensions{
                reason: format!("Cannot set the width or height of Button {:?} smaller than
                    the space taken by the button's text", self.id())
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;
        self.center_label_text();

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
        self.label.translate(dest);
    }

    /// convert to EmitEvent
    fn as_emit_event(&mut self) -> Option<&mut dyn context::EmitEvent> {
        Some(self)
    }

    /// Whether this widget accepts keyboard events
    fn accepts_keyboard_events(&self) -> bool {
        true
    }
}

impl_emit_event!(Button, self.handler_data);
widget_from_id!(Button);
