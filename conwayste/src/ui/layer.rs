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
    common::within_widget,
    widget::Widget,
    Pane,
    TextField,
    UIAction,
    UIError, UIResult,
    WidgetID,
    context,
};

use context::EmitEvent;

use crate::constants::colors::*;

pub struct Layer {
    pub id: WidgetID,
    pub widgets: Vec<Box<dyn Widget>>,
    pub with_transparency: bool,
    pub focused_widget: Option<WidgetID>,
    pub handlers: Option<context::HandlerMap>, // required for impl_emit_event!
    // option solely so that we can not mut borrow self twice at once
}

/// A container of one or more widgets or panes
impl Layer {
    /// Specify the unique widget identifer for the layer
    pub fn new(widget_id: WidgetID) -> Self {
        let mut layer = Layer {
            id: widget_id,
            widgets: vec![],
            with_transparency: false,
            focused_widget: None,
            handlers: Some(context::HandlerMap::new()),
        };

        // TODO: propagate events for other EventTypes
        let handler: context::Handler = Box::new(|obj, uictx, evt| {
            let layer = obj.downcast_mut::<Layer>().unwrap();
            use context::Handled::*;

            for w in layer.widgets.iter_mut() {
                if within_widget(&evt.point, &w.rect()) {
                    if let Some(obj) = w.as_emit_event() {
                        obj.emit(evt, uictx)?;
                        return Ok(Handled);
                    }
                }
            }

            Ok(NotHandled)
        });
        layer.on(context::EventType::Click, handler).unwrap(); // unwrap OK because we are not calling .on from within handler
        layer
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
    pub fn enter_focus(&mut self, id: WidgetID) -> UIResult<()> {
        self.exit_focus();

        if let Some(tf) = TextField::widget_from_id(self, id) {
            tf.enter_focus();
            self.focused_widget = Some(id);
            return Ok(());
        }

        let error_msg = format!("ERROR in Layer::enter_focus() =>  Widget (ID: {:?}) not found in layer (ID: {:?})",
            id,
            self.id
        );
        Err(Box::new(UIError::WidgetNotFound{reason: error_msg}))
    }

    /// Iterates through all of the widgets grouped in this layer searching for the specified WidgetID.
    /// If a Pane widget is found, it will search through all of its contained elements as well.
    /// In either scenario, the first element found will be returned.
    pub fn get_widget_mut(&mut self, id: WidgetID) -> UIResult<&mut Box<dyn Widget>>
    {
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
            let widget = pane.widgets.get_mut(p_i).unwrap();
            return Ok(widget);
        }
        if let Some(i) = index {
            // Unwrap safe because we found the index via enumerate
            let widget = self.widgets.get_mut(i).unwrap();
            return Ok(widget);
        }

        let error_msg = format!("ERROR in Layer::enter_focus() =>  Widget (ID: {:?}) not found in layer (ID: {:?})",
            id,
            self.id
        );
        Err(Box::new(UIError::WidgetNotFound{reason: error_msg}))
    }
}

impl Widget for Layer {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn rect(&self) -> Rect {
        Rect::zero()
    }

    fn position(&self) -> Point2<f32> { Point2::<f32>::new(0.0, 0.0) }
    fn set_position(&mut self, _x: f32, _y: f32) { }
    fn size(&self) -> (f32, f32) { (0.0, 0.0) }
    fn set_size(&mut self, _w: f32, _h: f32) -> UIResult<()> { Ok(()) }

    fn translate(&mut self, _dest: Vector2<f32>) { }

    fn on_hover(&mut self, point: &Point2<f32>) {
        for w in self.widgets.iter_mut() {
            w.on_hover(point);
        }
    }

    fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        for w in self.widgets.iter_mut() {
            if within_widget(point, &w.rect()) {
                let ui_action = w.on_click(point);
                if ui_action.is_some() {
                    return ui_action;
                }
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
                *LAYER_TRANSPARENCY_BG_COLOR,
            )?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        for widget in self.widgets.iter_mut() {
            widget.draw(ctx)?;
        }

        Ok(())
    }

    /// convert to EmitEvent
    fn as_emit_event(&mut self) -> Option<&mut dyn context::EmitEvent> {
        Some(self)
    }
}

impl_emit_event!(Layer, self.handlers);

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{chatbox::Chatbox, common::FontInfo, pane::Pane};
    use crate::constants;
    use ggez::graphics::Scale;

    fn create_dummy_layer() -> Layer {
        Layer::new(WidgetID(0))
    }

    fn create_dummy_font() -> FontInfo {
        FontInfo {
            font: (), //dummy font because we can't create a real Font without ggez
            scale: Scale::uniform(1.0), // Does not matter
            char_dimensions: Vector2::<f32>::new(5.0, 5.0),  // any positive values will do
        }
    }

    #[test]
    fn test_add_widget_to_layer_basic() {
        let mut layer = create_dummy_layer();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);

        layer.add(Box::new(chatbox));

        for (i, w) in layer.widgets.iter().enumerate() {
            assert_eq!(i, 0);
            assert_eq!(w.id(), WidgetID(0));
        }
    }

    #[test]
    fn test_add_widget_two_widget_share_the_same_id() {
        let mut layer = create_dummy_layer();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        layer.add(Box::new(chatbox));

        // TODO: This should fail because a WidgetID(0) already is present in the layer
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        layer.add(Box::new(chatbox));
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists() {
        let mut layer = create_dummy_layer();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);

        layer.add(Box::new(chatbox));
        let w = layer.get_widget_mut(WidgetID(0)).unwrap();
        assert_eq!(w.id(), WidgetID(0));
    }

    #[test]
    fn test_get_widget_mut_widget_does_not_exist_list_is_empty() {
        let mut layer = create_dummy_layer();

        assert!(layer.get_widget_mut(WidgetID(0)).is_err());
    }

    #[test]
    fn test_get_widget_mut_widget_does_not_exist_list_non_empty() {
        let mut layer = create_dummy_layer();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(1), font_info, history_len);

        layer.add(Box::new(chatbox));
        let _w = layer.get_widget_mut(WidgetID(0));
    }

    #[test]
    fn test_get_widget_mut_widget_is_a_pane() {
        let mut layer = create_dummy_layer();
        let pane = Pane::new(WidgetID(0), Rect::new(0.0, 0.0, 100.0, 100.0));

        layer.add(Box::new(pane));
        let w = layer.get_widget_mut(WidgetID(0)).unwrap();
        assert_eq!(w.id(), WidgetID(0));
    }

    #[test]
    fn test_get_widget_mut_widget_is_within_a_pane() {
        let mut layer = create_dummy_layer();
        let mut pane = Pane::new(WidgetID(0), *constants::DEFAULT_CHATBOX_RECT);
        let font_info = create_dummy_font();
        let history_len = 5;
        let mut chatbox = Chatbox::new(WidgetID(1), font_info, history_len);
        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(pane.add(Box::new(chatbox)).is_ok());
        layer.add(Box::new(pane));
        let w = layer.get_widget_mut(WidgetID(1)).unwrap();
        assert_eq!(w.id(), WidgetID(1));
    }

}
