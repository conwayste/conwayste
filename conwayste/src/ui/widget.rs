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

use ggez::graphics::Rect;
use ggez::mint::{Point2, Vector2};
use ggez::{Context, GameResult};

use downcast_rs::Downcast;

use id_tree::NodeId;

use super::{context, UIResult};

/// A user interface element trait that defines graphical, interactive behavior to be specified.
/// Relies on the `downcast_rs` crate to be able to transform widgets into their specific
/// implementations.
///
/// When defining your own Widget, be sure to call the widget_from_id!(T) macro where T is your
/// custom widget type.
pub trait Widget: Downcast + std::fmt::Debug {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> Option<&NodeId>;

    fn set_id(&mut self, new_id: NodeId);

    /// Retreives the widget's draw stack order
    fn z_index(&self) -> usize;

    /// Sets the widget's draw stack order. Normally this is set when this widget is provided
    /// to Layering::add_widget.
    fn set_z_index(&mut self, _new_z_index: usize) {
        ()
    }

    /// Called upon each graphical draw tick. This should be where the widget's graphics takes place.
    fn draw(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    /// Get the rectangle describing the widget.
    fn rect(&self) -> Rect;

    /// Get the origin point of the widget in screen coordinates.
    fn position(&self) -> Point2<f32>;

    /// Get the width and height of the widget in pixels.
    fn size(&self) -> (f32, f32);

    /// Set the size of the widget.
    fn set_rect(&mut self, _new_dimensions: Rect) -> UIResult<()> {
        Ok(())
    }

    /// Set the size of the widget.
    fn set_position(&mut self, _x: f32, _y: f32) {
        ()
    }

    fn set_size(&mut self, _w: f32, _h: f32) -> UIResult<()> {
        Ok(())
    }

    /// Translate the widget from one location to another.
    fn translate(&mut self, _dest: Vector2<f32>);

    /// If the widget implements EmitEvent, implementors should have this return Some(self) here.
    /// NOTE: we wouldn't need this if our downcasting crate was smarter.
    fn as_emit_event(&mut self) -> Option<&mut dyn context::EmitEvent> {
        None
    }

    /// Whether this widget accepts keyboard events
    fn accepts_keyboard_events(&self) -> bool {
        false
    }
}

impl_downcast!(Widget);
