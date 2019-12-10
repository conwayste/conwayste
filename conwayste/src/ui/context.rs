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

use ggez;
use crate::config;

pub enum UIContext<'a> {
    Draw(DrawContext<'a>),
    Update(UpdateContext<'a>),
}

impl<'a> UIContext<'a> {
    fn unwrap_draw(&mut self) -> &mut DrawContext<'a> {
        match *self {
            UIContext::Draw(ref mut draw_context) => draw_context,
            _ => panic!("Failed to unwrap DrawContext"),
        }
    }

    fn unwrap_update(&mut self) -> &mut UpdateContext<'a> {
        match *self {
            UIContext::Update(ref mut update_context) => update_context,
            _ => panic!("Failed to unwrap UpdateContext"),
        }
    }
}

pub struct DrawContext<'a> {
    pub ggez_context: &'a mut ggez::Context,
    pub config: &'a config::Config,
}

pub struct UpdateContext<'a> {
    pub ggez_context: &'a mut ggez::Context,
    pub config: &'a mut config::Config,
}
