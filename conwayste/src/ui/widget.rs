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
use ggez::graphics::{Rect};
use ggez::nalgebra::{Point2, Vector2};

use downcast_rs::Downcast;

use super::{UIAction,WidgetID};

/// A user interface element trait that defines graphical, interactive behavior to be specified.
/// Relies on the `downcast_rs` crate to be able to transform widgets into their specific implementations.
pub trait Widget: Downcast {
    /// Retrieves the widget's unique identifer
    fn id(&self) -> WidgetID;

    /// Action to be taken when the widget is given the provided point
    fn on_hover(&mut self, _point: &Point2<f32>) {
        ()
    }

    /// Action to be taken when the widget is given the provided point, and a mouse click occurs
    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
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

    // Long-term TODO
    // The following implementaions tend to use struct member variables so we need to define their own definitions
    // Refactor into a Sizable trait

    /// Get the size of the widget. Widget must be sizable.
    fn size(&self) -> Rect;

    /// Set the size of the widget. Widget must be sizable.
    fn set_size(&mut self, _new_dimensions: Rect) {
        ()
    }

    /// Translate the widget from one location to another. Widget must be sizable.
    fn translate(&mut self, _dest: Vector2<f32>);
}

impl_downcast!(Widget);