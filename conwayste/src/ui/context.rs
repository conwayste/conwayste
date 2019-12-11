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

use crate::config;
use ggez;

pub enum UIContext<'a> {
    Draw(DrawContext<'a>),
    Update(UpdateContext<'a>),
}

impl<'a> UIContext<'a> {
    pub fn unwrap_draw(&mut self) -> &mut DrawContext<'a> {
        match *self {
            UIContext::Draw(ref mut draw_context) => draw_context,
            _ => panic!("Failed to unwrap DrawContext"),
        }
    }

    pub fn unwrap_update(&mut self) -> &mut UpdateContext<'a> {
        match *self {
            UIContext::Update(ref mut update_context) => update_context,
            _ => panic!("Failed to unwrap UpdateContext"),
        }
    }

    pub fn new_draw(ggez_context: &'a mut ggez::Context, config: &'a config::Config) -> Self {
        UIContext::Draw(DrawContext {
            ggez_context,
            config,
        })
    }

    pub fn new_update(ggez_context: &'a mut ggez::Context, config: &'a mut config::Config) -> Self {
        UIContext::Update(UpdateContext {
            ggez_context,
            config,
        })
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
