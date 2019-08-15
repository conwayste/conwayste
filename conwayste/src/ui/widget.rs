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
use ggez::graphics::{Font, Rect};
use ggez::nalgebra::{Point2, Vector2};

use downcast_rs::Downcast;

use super::{UIAction,WidgetID};

pub trait Widget: Downcast {
    fn on_hover(&mut self, _point: &Point2<f32>) {
        ()
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        None
    }

    fn on_drag(&mut self, _original_point: &Point2<f32>, _point: &Point2<f32>) {
        ()
    }

    fn draw(&mut self, _ctx: &mut Context, _font: &Font) -> GameResult<()> {
        Ok(())
    }

    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    // The following implementaions tend to use struct member variables so we need to define their own definitions

    fn size(&self) -> Rect;
    fn set_size(&mut self, new_dimensions: Rect);
    fn translate(&mut self, point: Vector2<f32>);
    fn id(&self) -> WidgetID;
}

impl_downcast!(Widget);