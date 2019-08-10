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
use ggez::graphics::{self, Rect, Font, Point2, Color, DrawMode, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    helpe::color_with_alpha,
    Chatbox,
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

impl Layer {
    pub fn new(widget_id: WidgetID) -> Self {
        Layer {
            id: widget_id,
            widgets: vec![],
            with_transparency: false,
            focused_widget: None,
        }
    }

    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    pub fn exit_focus(&mut self) {
        if let Some(other_id) = self.focused_widget {
            if let Some(other_tf) = self.textfield_from_id(other_id) {
                other_tf.exit_focus();
            }
        }
    }

    pub fn enter_focus(&mut self, id: WidgetID) {
        self.exit_focus();

        if let Some(tf) = self.textfield_from_id(id) {
            tf.enter_focus();
            self.focused_widget = Some(id);
            return;
        }

        // FIXME Replace with ConwaysteResult
        panic!("ERROR in Layer::enter_focus() =>  Widget (ID: {:?}) not found in layer (ID: {:?})", id, self.id);
    }

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

        // FIXME Replace with ConwaysteResult
        panic!("ERROR in Layer::get_widget_mut() => Widget (ID: {:?}) not found in layer (ID: {:?})", id, self.id);
    }

    pub fn textfield_from_id(&mut self, id: WidgetID) -> Option<&mut TextField> {
        let widget = self.get_widget_mut(id);
        return widget.downcast_mut::<TextField>();
    }

    pub fn chatbox_from_id(&mut self, id: WidgetID) -> Option<&mut Chatbox> {
        let widget = self.get_widget_mut(id);
        return widget.downcast_mut::<Chatbox>();
    }
}

impl Widget for Layer {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        Rect::zero()
    }

    fn set_size(&mut self, _new_dimensions: Rect) {}

    fn translate(&mut self, _point: Vector2) {}

    fn on_hover(&mut self, point: &Point2) {
        for w in self.widgets.iter_mut() {
            w.on_hover(point);
        }
    }

    fn on_click(&mut self, point: &Point2) -> Option<(WidgetID, UIAction)> {
        for w in self.widgets.iter_mut() {
            let ui_action = w.on_click(point);
            if ui_action.is_some() {
                return ui_action;
            }
        }
        None
    }

    fn on_drag(&mut self, original_pos: &Point2, current_pos: &Point2) {
        for w in self.widgets.iter_mut() {
            w.on_drag(original_pos, current_pos);
        }
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let old_color = graphics::get_color(ctx);

        if self.with_transparency {
            // TODO wait for winint to get resolution
            graphics::set_color(ctx, color_with_alpha(css::HONEYDEW, 0.1))?;
            graphics::rectangle(ctx, DrawMode::Fill, Rect::new(0.0, 0.0, 1920.0, 1080.0))?;
        }

        for widget in self.widgets.iter_mut() {
            widget.draw(ctx, font)?;
        }

        graphics::set_color(ctx, old_color)?;

        Ok(())
    }
}