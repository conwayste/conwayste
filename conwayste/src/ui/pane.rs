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

use ggez::graphics::{self, Rect, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    common::{within_widget},
    UIAction, WidgetID
};

use crate::constants::colors::*;

pub struct Pane {
    pub id: WidgetID,
    pub dimensions: Rect,
    pub widgets: Vec<Box<dyn Widget>>,
    pub hover: bool,
    pub floating: bool, // can the window be dragged around?
    pub previous_pos: Option<Point2<f32>>,

    // might need something to track mouse state to see if we are still clicked within the
    // boundaries of the pane in the dragging case
}

/// A container of one or more widgets
impl Pane {
    /// Specify the unique widget identifer for the pane, and its dimensional bounds
    pub fn new(widget_id: WidgetID, dimensions: Rect) -> Self {
        Pane {
            id: widget_id,
            dimensions: dimensions,
            widgets: vec![],
            hover: false,
            floating: true,
            previous_pos: None,
        }
    }

    /// Add a widget to the pane
    /// TODO: ensure that the widget being added fits inside the pane's bounds
    pub fn add(&mut self, mut widget: Box<dyn Widget>) {
        let dims = widget.size();
        widget.set_size(Rect::new(dims.x + self.dimensions.x, dims.y + self.dimensions.y, dims.w, dims.h));
        self.widgets.push(widget);
    }

    /*
    // TODO: Currently used to reset previous position on mouse release after dragging completes.
    //      Re-evaluate design if this is the best way to do it. See issue #71 (dragging).
    pub fn update(&mut self, is_mouse_released: bool) {
        if is_mouse_released {
            self.previous_pos = None;
        }
    }
    */
}

impl Widget for Pane {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
    }

    fn translate(&mut self, dest: Vector2<f32>)
    {
        self.dimensions.translate(dest);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        for w in self.widgets.iter_mut() {
            w.on_hover(point);
        }
    }

    fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let hover = self.hover;
        self.hover = false;

        if hover {
            for w in self.widgets.iter_mut() {
                let ui_action = w.on_click(point);
                if ui_action.is_some() {
                    return ui_action;
                }
            }
        }
        None
    }


    /* TODO: fix all the drag issues
    /// original_pos is the mouse position at which the button was held before any dragging occurred
    /// current_pos is the latest mouse position after any movement
    fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {

        if !self.floating || !self.hover {
            return;
        }

        let mut drag_ok = true;

        // Check that the mouse down event is bounded by the pane but not by a sub-widget
        if within_widget(original_pos, &self.dimensions) {
            for widget in self.widgets.iter() {
                if within_widget(original_pos, &widget.size()) && self.previous_pos.is_none() {
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
    */

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let mesh = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), self.dimensions, *PANE_BG_FILL_COLOR)?;
        graphics::draw(ctx, &mesh, DrawParam::default())?;

        for widget in self.widgets.iter_mut() {
            widget.draw(ctx)?;
        }

        Ok(())
    }
}

widget_from_id!(Pane);
