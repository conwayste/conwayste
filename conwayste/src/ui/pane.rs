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

use ggez::graphics::{self, Rect, Font, Text, Point2, Color, DrawMode, Vector2};
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
    pub previous_pos: Option<Point2>,

    // might need something to track mouse state to see if we are still clicked within the boundaries of the pane for dragging
}

impl Pane {
    pub fn new(dimensions: Rect) -> Self {
        Pane {
            dimensions: dimensions,
            widgets: vec![],
            hover: false,
            floating: true,
            previous_pos: None,
        }
    }

    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    pub fn update(&mut self, is_mouse_released: bool) {
        if is_mouse_released {
            self.previous_pos = None;
        }
    }
}

impl Widget for Pane {
    fn dimensions(&self) -> Rect {
        self.dimensions
    }

    fn set_dimensions(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
    }

    fn translate(&mut self, point: Vector2)
    {
        self.dimensions.translate(point);
    }

    fn on_hover(&mut self, point: &Point2) {
        self.hover = within_widget(point, &self.dimensions);
    }

    fn on_click(&mut self, _point: &Point2) -> Option<UserAction> {

        None
    }

    fn on_drag(&mut self, original_pos: &Point2, current_pos: &Point2) {

        if !self.floating {
            return;
        }

        let mut drag_ok = true;

        // Check that the mouse down event is bounded by the pane but not by a sub-widget
        if within_widget(original_pos, &self.dimensions) {
            for widget in self.widgets.iter() {
                if within_widget(original_pos, &widget.dimensions()) && self.previous_pos.is_none() {
                    drag_ok = false;
                    break;
                }
            }
        } else {
            // The original mouse down event may be no longer bounded if the pane moved enough,
            // so check if we were dragging at a previous spot
            drag_ok = self.previous_pos.is_some();
        }

        if drag_ok {
            // Note where the pane was previously to calculate the delta in position
            if let None = self.previous_pos {
                self.previous_pos = Some(*current_pos);
            }

            if let Some(origin) = self.previous_pos {
                let tl_corner_offset = current_pos - origin;

                if tl_corner_offset[0] != 0.0 && tl_corner_offset[1] != 0.0 {
                    //println!("Dragging! {}, {}, {}", origin, current_pos, tl_corner_offset);
                }

                self.translate(tl_corner_offset);
                for ref mut widget in self.widgets.iter_mut() {
                    widget.translate(tl_corner_offset);
                }
            }

            self.previous_pos = Some(*current_pos);
        }
    }

    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let old_color = graphics::get_color(ctx);

        graphics::set_color(ctx, Color::from(css::WHITE))?;
        graphics::rectangle(ctx, DrawMode::Line(4.0), self.dimensions)?;

        for widget in self.widgets.iter() {
            widget.draw(ctx, font)?;
        }

        graphics::set_color(ctx, old_color)?;

        Ok(())
    }
}