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

use ggez::{Context, GameResult};
use ggez::graphics::Rect;
use ggez::nalgebra::{Point2, Vector2};

use downcast_rs::Downcast;

use id_tree::NodeId;

use super::{
    UIAction,
    UIResult,
    context,
};

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

    /// Action to be taken when the widget is given the provided point
    fn on_hover(&mut self, _point: &Point2<f32>) {
        ()
    }

    /// Action to be taken when the widget is given the provided point, and a mouse click occurs
    fn on_click(&mut self, _point: &Point2<f32>) -> Option<UIAction> {
        None
    }

    /// Action to be taken when the widget is interacted with while the mouse is clicked-and-held
    /// and dragged around the screen.
    ///
    /// # Arguments
    /// * `original_point` - the point at which the dragging began
    /// * `point` - the current position of the mouse cursor
    fn on_drag(&mut self, _original_point: &Point2<f32>, _point: &Point2<f32>) {
        ()
    }

    /// Called upon each graphical draw tick. This should be where the widget's graphics takes place.
    fn draw(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    /// Called upon each logic update tick. This should be where the widget's logic takes place.
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    // TODO: delete
    /// Widget gains focus and begins accepting user input
    fn enter_focus(&mut self) {
        ()
    }

    // TODO: delete
    /// Widget loses focus and does not accept user input
    fn exit_focus(&mut self) {
        ()
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

    //XXX HACK
    fn as_emit_event(&mut self) -> Option<&mut dyn context::EmitEvent> {
        None
    }

    /// Whether this widget accepts keyboard events
    fn accepts_keyboard_events(&self) -> bool {
        false
    }
}

impl_downcast!(Widget);
