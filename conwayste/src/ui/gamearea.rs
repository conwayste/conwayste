/*  Copyright 2020 the Conwayste Developers.
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

use super::{
    context::{EmitEvent, Event, EventType, Handled, HandlerData, UIContext},
    widget::Widget,
    UIError, UIResult,
};
use ggez::graphics::Rect;
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};
use id_tree::NodeId;
use std::error::Error;

#[derive(Debug)]
pub struct GameArea {
    id: Option<NodeId>,
    pub has_keyboard_focus: bool,
    z_index: usize,
    dimensions: Rect,
    handler_data: HandlerData,
}

/// For now, this is a dummy widget to represent the actual game area. It may not always be a dummy
/// widget.
impl GameArea {
    pub fn new() -> Self {
        let mut game_area = GameArea {
            id: None,
            has_keyboard_focus: false,
            z_index: 0,
            dimensions: Rect::default(),
            handler_data: HandlerData::new(),
        };

        // Set handlers for toggling has_keyboard_focus
        let gain_focus_handler = move |obj: &mut dyn EmitEvent,
                                       _uictx: &mut UIContext,
                                       _evt: &Event|
              -> Result<Handled, Box<dyn Error>> {
            let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
            game_area.has_keyboard_focus = true;
            Ok(Handled::NotHandled)
        };

        let lose_focus_handler = move |obj: &mut dyn EmitEvent,
                                       _uictx: &mut UIContext,
                                       _evt: &Event|
              -> Result<Handled, Box<dyn Error>> {
            let game_area = obj.downcast_mut::<GameArea>().unwrap(); // unwrap OK
            game_area.has_keyboard_focus = false;
            Ok(Handled::NotHandled)
        };

        game_area
            .on(EventType::GainFocus, Box::new(gain_focus_handler))
            .unwrap(); // unwrap OK
        game_area
            .on(EventType::LoseFocus, Box::new(lose_focus_handler))
            .unwrap(); // unwrap OK

        game_area
    }
}

impl Widget for GameArea {
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
                reason: format!(
                    "Cannot set the size to a width or height of GameArea {:?} to zero",
                    self.id()
                ),
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
                reason: format!(
                    "Cannot set the width or height of GameArea {:?} to zero",
                    self.id()
                ),
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
    }

    fn draw(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // no-op; dummy widget
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

impl_emit_event!(GameArea, self.handler_data);
widget_from_id!(GameArea);
