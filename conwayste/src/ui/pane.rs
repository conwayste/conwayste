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

use chromatica::css;

use ggez::graphics::{self, Rect, Font, Text, Point2, Color, DrawMode};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text},
    UserAction
    };

pub struct Pane {
    pub dimensions: Rect,
    pub widgets: Vec<Box<dyn Widget>>,
    pub hover: bool,
    pub floating: bool, // can the window be dragged around?
    pub drag_origin: Option<Point2>,

    // might need something to track mouse state to see if we are still clicked within the boundaries of the pane for dragging
}

impl Pane {
    pub fn new(dimensions: Rect) -> Self {
        Pane {
            dimensions: dimensions,
            widgets: vec![],
            hover: false,
            floating: true,
            drag_origin: None,
        }
    }

    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    pub fn update(&mut self, is_mouse_released: bool) {
        if is_mouse_released {
            self.drag_origin = None;
        }
    }
}

impl Widget for Pane {
    fn on_hover(&mut self, point: &Point2) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn on_click(&mut self, _point: &Point2) -> Option<UserAction> {

        None
    }

    fn on_drag(&mut self, point: &Point2) {

        // TODO handle outside of pane dragging in
        // TODO handle inside pane dragging out

        if !self.floating {
            return;
        }

        if let None = self.drag_origin {
            self.drag_origin = Some(*point);
        }

        let mut drag_ok = true;
        if within_widget(point, &self.dimensions) {
            for widget in self.widgets.iter() {
                if within_widget(point, &widget.dimensions()) {
                    drag_ok = false;
                    break;
                }
            }
        } else {
            drag_ok = false;
        }

        if drag_ok {
            if let Some(origin) = self.drag_origin {
                let tl_corner_offset = point - origin;

                if tl_corner_offset[0] != 0.0 && tl_corner_offset[1] != 0.0 {
                    println!("Dragging! {}, {}, {}", origin, point, tl_corner_offset);
                }

                self.dimensions.translate(tl_corner_offset);
            }
            self.drag_origin = Some(*point);
        }
    }

    fn draw(&self, ctx: &mut Context, _font: &Font) -> GameResult<()> {
        let old_color = graphics::get_color(ctx);

        graphics::set_color(ctx, Color::from(css::WHITE))?;
        graphics::rectangle(ctx, DrawMode::Line(4.0), self.dimensions)?;

        graphics::set_color(ctx, old_color)?;

        Ok(())
    }
}