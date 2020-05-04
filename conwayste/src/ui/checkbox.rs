
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

use ggez::graphics::{self, Rect, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};
use ggez::input::keyboard::KeyCode;
use ggez::event::MouseButton;

use id_tree::NodeId;

use super::{
    label::Label,
    widget::Widget,
    common::{within_widget, FontInfo},
    UIError,
    UIResult,
};
use super::context::{
    EmitEvent,
    Event,
    EventType,
    Handled,
    HandlerData,
    UIContext,
};

use crate::constants::colors::*;

pub struct Checkbox {
    id: Option<NodeId>,
    z_index: usize,
    pub label: Label,
    pub enabled: bool,
    pub dimensions: Rect,
    pub focused: bool, // has keyboard focus?
    pub draw_hover: bool, //XXX delete this
    pub handler_data: HandlerData, // required for impl_emit_event!
}

impl fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Checkbox {{ id: {:?}, z-index: {}, Dimensions: {:?}, Checked: {} }}",
            self.id, self.z_index, self.dimensions, self.enabled)
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
        enabled: bool,
        font_info: FontInfo,
        text: String,
        dimensions: Rect,
    ) -> Self {
        let label_origin = Point2::new(
            dimensions.x + dimensions.w + LABEL_OFFSET_X,
            dimensions.y + dimensions.h + LABEL_OFFSET_Y
        );

        let mut cb = Checkbox {
            id: None,
            z_index: std::usize::MAX,
            label: Label::new(ctx, font_info, text, *CHECKBOX_TEXT_COLOR, label_origin),
            enabled,
            dimensions,
            focused: false,
            draw_hover: false,
            handler_data: HandlerData::new(),
        };

        // setup handler to allow changing appearance when it has keyboard focus
        cb.on(EventType::GainFocus, Box::new(Checkbox::focus_change_handler)).unwrap(); // unwrap OK b/c not being called within handler
        cb.on(EventType::LoseFocus, Box::new(Checkbox::focus_change_handler)).unwrap(); // unwrap OK b/c not being called within handler

        cb.on(EventType::Click, Box::new(Checkbox::click_handler)).unwrap();

        // setup handler to forward a space keyboard event to the click handler
        cb.on(EventType::KeyPress, Box::new(Checkbox::keypress_handler)).unwrap(); // unwrap OK b/c not being called within handler

        cb
    }

    fn focus_change_handler(obj: &mut dyn EmitEvent, _uictx: &mut UIContext, event: &Event) -> Result<Handled, Box<dyn Error>> {
        let checkbox = obj.downcast_mut::<Checkbox>().unwrap(); // unwrap OK because this will always be Checkbox
        match event.what {
            EventType::GainFocus => checkbox.focused = true,
            EventType::LoseFocus => checkbox.focused = false,
            _ => unimplemented!("this handler is only for gaining/losing focus"),
        };
        Ok(Handled::NotHandled) // allow other handlers for this event type to be activated
    }

    fn keypress_handler(obj: &mut dyn EmitEvent, uictx: &mut UIContext, event: &Event) -> Result<Handled, Box<dyn Error>> {
        let checkbox = obj.downcast_mut::<Checkbox>().unwrap(); // unwrap OK because this will always be Checkbox
        if Some(KeyCode::Space) != event.key {
            return Ok(Handled::NotHandled);
        }
        // create a synthetic click event
        let mouse_point = checkbox.position();
        let click_event = Event {
            what: EventType::Click,
            point: Some(mouse_point),
            prev_point: None,
            button: Some(MouseButton::Left),
            key: None,
            shift_pressed: false,
        };
        checkbox.emit(&click_event, uictx)?;
        Ok(Handled::NotHandled) // allow other handlers for this event type to be activated
    }

    fn click_handler(obj: &mut dyn EmitEvent, _uictx: &mut UIContext, _evt: &Event) -> Result<Handled, Box<dyn Error>> {
        let mut checkbox = obj.downcast_mut::<Checkbox>().unwrap();

        // toggle
        checkbox.enabled = !checkbox.enabled;

        Ok(Handled::NotHandled)
    }
}


impl Widget for Checkbox {
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
        self.draw_hover = within_widget(point, &self.dimensions) || within_widget(point, &label_dimensions);
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {

        if self.draw_hover || self.focused {
            // Add in a violet border/fill while hovered. Color checkbox differently to indicate
            // hovering and/or keyboard focus.
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

    /// convert to EmitEvent
    fn as_emit_event(&mut self) -> Option<&mut dyn EmitEvent> {
        Some(self)
    }

    /// Whether this widget accepts keyboard events
    fn accepts_keyboard_events(&self) -> bool {
        true
    }
}

impl_emit_event!(Checkbox, self.handler_data);
widget_from_id!(Checkbox);
