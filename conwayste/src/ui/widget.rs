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
use ggez::graphics::{Font, Point2, Rect};

use super::UserAction;

pub trait Widget {
    fn on_hover(&mut self, _point: &Point2) {
        ()
    }

    fn on_click(&mut self, _point: &Point2) -> Option<UserAction> {
        None
    }

    fn on_drag(&mut self, _point: &Point2) {
        ()
    }

    fn draw(&self, _ctx: &mut Context, _font: &Font) -> GameResult<()> {
        Ok(())
    }

    fn dimensions(&self) -> Rect {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    }
}
