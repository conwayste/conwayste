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
use ggez::graphics::{self, Rect, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    common::color_with_alpha,
    Pane,
    TextField,
    UIAction,
    WidgetID
};

pub struct Layer {
    pub id: WidgetID,
    pub widgets: Vec<Box<dyn Widget>>,
    pub with_transparency: bool,
    pub focused_widget: Option<WidgetID>,
}

/// A container of one or more widgets or panes
impl Layer {
    /// Specify the unique widget identifer for the layer
    pub fn new(widget_id: WidgetID) -> Self {
        Layer {
            id: widget_id,
            widgets: vec![],
            with_transparency: false,
            focused_widget: None,
        }
    }

    /// Add a widget to the layer
    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    /// Layer passes `exit_focus` request forward to its elements
    pub fn exit_focus(&mut self) {
        if let Some(other_id) = self.focused_widget {
            if let Some(other_tf) = TextField::widget_from_id(self, other_id)
            {
                other_tf.exit_focus();
            }
        }
    }

    /// Layer passes `enter_focus` request forward to its elements
    pub fn enter_focus(&mut self, id: WidgetID) {
        self.exit_focus();

        if let Some(tf) = TextField::widget_from_id(self, id) {
            tf.enter_focus();
            self.focused_widget = Some(id);
            return;
        }

        // PR_GATE Replace with ConwaysteResult
        panic!("ERROR in Layer::enter_focus() =>  Widget (ID: {:?}) not found in layer (ID: {:?})", id, self.id);
    }

    /// Iterates through all of the widgets grouped in this layer searching for the specified WidgetID.
    /// If a Pane widget is found, it will search through all of its contained elements as well.
    /// In either scenario, the first element found will be returned.
    pub fn get_widget_mut(&mut self, id: WidgetID) -> &mut Box<dyn Widget> {
        let mut index = None;
        let mut pane_index = None;

        for (i, w) in self.widgets.iter().enumerate() {
            if w.id() == id {
                index = Some(i);
                break;
            }

            if let Some(pane) = w.downcast_ref::<Pane>() {
                for (j, x) in pane.widgets.iter().enumerate() {
                    if x.id() == id {
                        index = Some(i);
                        pane_index = Some(j);
                    }
                }
            }
        }

        if let Some(p_i) = pane_index {
            let i = index.unwrap();

            let pane = self.widgets.get_mut(i).unwrap().downcast_mut::<Pane>().unwrap();
            return pane.widgets.get_mut(p_i).unwrap();
        }
        if let Some(i) = index {
            // Unwrap safe because we found the index via enumerate
            return self.widgets.get_mut(i).unwrap();
        }

        // PR_GATE Replace with ConwaysteResult
        panic!("ERROR in Layer::get_widget_mut() => Widget (ID: {:?}) not found in layer (ID: {:?})", id, self.id);
    }
}

impl Widget for Layer {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        Rect::zero()
    }

    fn translate(&mut self, _dest: Vector2<f32>) {}

    fn on_hover(&mut self, point: &Point2<f32>) {
        for w in self.widgets.iter_mut() {
            w.on_hover(point);
        }
    }

    fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        for w in self.widgets.iter_mut() {
            let ui_action = w.on_click(point);
            if ui_action.is_some() {
                return ui_action;
            }
        }
        None
    }

    fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {
        for w in self.widgets.iter_mut() {
            w.on_drag(original_pos, current_pos);
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.with_transparency {
            // TODO: Get resolution from video-settings
            let mesh = graphics::Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(0.0, 0.0, 1920.0, 1080.0),
                color_with_alpha(css::HONEYDEW, 0.4)
            )?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        for widget in self.widgets.iter_mut() {
            widget.draw(ctx)?;
        }

        Ok(())
    }
}
