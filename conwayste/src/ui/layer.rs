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
use ggez::nalgebra::Point2;
use ggez::Context;

use super::{
    widget::Widget,
    Pane,
    TextField,
    UIAction,
    UIError, UIResult,
    WidgetID
};

use crate::constants::colors::*;

pub struct Layering {
    pub with_transparency: bool,
    pub widget_list: Vec<Box<dyn Widget>>,
    focused_ids: Vec<Option<WidgetID>>,
    id_cache: Vec<WidgetID>
}

/// A Layering is a container of one or more widgets or panes, where widgets are ordered by
/// by their z_index to create the appearance of a layer, for a given game screen. Layerings
/// support an optional transparency between two adjacent z-orders.
impl Layering {
    pub fn new() -> Self {
        Layering {
            with_transparency: false,
            widget_list: vec![],
            focused_ids: vec![None],
            id_cache: vec![]
        }
    }

    /// Returns true if an entry with the provided WidgetID exists.
    fn check_for_entry(&self, widget_id: WidgetID) -> bool {
        self.id_cache.iter().find(|&&id| id == widget_id).is_some()
    }

    /// Returns an optional pair of indices if the widget-id is found in Pane beloning to
    /// the layering.
    fn search_panes_for_widget_id(&self, widget_id: WidgetID) -> Option<(usize, usize)> {
        // First check to see if it belongs to any pane
        for (i, w) in self.widget_list.iter().enumerate() {
            if let Some(pane) = downcast_widget!(w, Pane) {
                for (j, w2) in pane.widgets.iter().enumerate() {
                    if w2.id() == widget_id {
                        return Some((i, j));
                    }
                }
            }
        }

        None
    }

    /// Caches the widget-id's residing in this layering. Will include widgets belonging to a one
    /// level deep pane.
    fn rebuild_id_cache(&mut self) {
        let mut local_id_cache = vec![];
        for widget in self.widget_list.iter() {
            local_id_cache.push(widget.id());

            if let Some(pane) = downcast_widget!(widget, Pane) {
                local_id_cache.extend(pane.get_widget_ids());
            }
        }
        self.id_cache = local_id_cache;
    }

    /// Retreives a mutable reference to a widget. This will search one Pane-level deep
    /// for the provided widget-id.
    ///
    /// # Error
    /// A WidgetNotFound error will be returned if the widget-id is not found.
    /// does not exist in the internal list of widgets.
    pub fn get_widget_mut(&mut self, widget_id: WidgetID) -> UIResult<&mut Box<dyn Widget>> {
        if let Some((list_index, pane_index)) = self.search_panes_for_widget_id(widget_id) {
            // Unwraps are safe because the previous search would return None if it couldn't find
            // a pane.
            let pane = self.widget_list.get_mut(list_index).unwrap().downcast_mut::<Pane>().unwrap();
            let widget = pane.widgets.get_mut(pane_index).unwrap();
            return Ok(widget);
        }

        // If it doesn't belong to a pane, check the widget list for an entry
        self.widget_list.iter_mut()
            .filter(|widget| widget.id() == widget_id)
            .next()
            .ok_or_else(|| Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list", widget_id).to_owned()
            }))
    }

    /// Add a widget to the layering at the provided z_index depth. Internal data structure
    /// maintains a list sorted by descending z_index.
    ///
    /// # Error
    /// A WidgetIDCollision error can be returned if the widget-id exists in this layering.
    pub fn add_widget(&mut self, widget: Box<dyn Widget>, z_index: usize) -> UIResult<()> {
        if self.check_for_entry(widget.id()) {
            return Err(Box::new(UIError::WidgetIDCollision {
                    reason: format!("Widget with ID {:?} exists in layer's widget list.", widget.id())
            }));
        }

        // Insert by z-index, descending. Use zero when the list is empty.
        let insertion_index = self.widget_list.iter()
            .position(|widget| z_index >= widget.z_index())
            .unwrap_or(0);

        self.widget_list.insert(insertion_index, widget);
        self.rebuild_id_cache();
        Ok(())
    }

    /// Removes a widget belonging to the layering
    ///
    /// # Error
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist
    /// in the internal list of widgets.
    pub fn _remove_widget(&mut self, widget_id: WidgetID) -> UIResult<()> {
        let removal_index = self.widget_list.iter()
            .position(|widget| widget_id >= widget.id())
            .ok_or_else(|| -> UIResult<()> {
                return Err(Box::new(UIError::WidgetNotFound {
                    reason: format!("{:?} not found in layer during removal", widget_id).to_owned()
                }));
            }).unwrap(); // unwrap safe because we must have an Ok(...) if we did not return

        let widget = self.widget_list.remove(removal_index);
        if let Some(id) = self.focused_ids[widget.z_index()] {
            if widget_id == id {
                self.focused_ids[widget.z_index()] = None;
            }
        }

        self.rebuild_id_cache();
        Ok(())
    }

    /// Returns the WidgetID of the widget currently in-focus
    pub fn focused_widget_id(&self) -> Option<WidgetID> {
        return self.focused_ids.last().map_or(None, |opt_id| *opt_id);
    }

    /// Retreive the z-index of the first widget in the widget list. Defaults to the zeroth depth if
    /// widget list is empty.
    fn peek_z_index(&self) -> usize {
        self.widget_list.first().map(|widget| widget.z_index()).unwrap_or_default()
    }

    /// Notifies the layer that the provided WidgetID is to hold focus.widget
    ///
    /// # Error
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist in
    /// the internal list of widgets.
    pub fn enter_focus(&mut self, widget_id: WidgetID) -> UIResult<()> {
        let mut indices = None;
        if !self.check_for_entry(widget_id) {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list", widget_id)
            }));
        }

        if let Some((list_i, pane_i)) = self.search_panes_for_widget_id(widget_id) {
            indices = Some((list_i, pane_i));
        }

        if let Some((list_index, pane_index)) = indices {
            // Found in a Pane. Unwraps below are safe because of check_for_entry call
            let dyn_widget = self.widget_list.get_mut(list_index).unwrap();
            let pane = downcast_widget!(mut dyn_widget, Pane).unwrap();
            let widget = pane.widgets.get_mut(pane_index).unwrap();
            widget.enter_focus();
        } else {
            // unwrap safe because of check_for_entry call
            let widget = self.widget_list.iter_mut()
                .filter(|widget| widget.id() == widget_id)
                .next().
                unwrap();
            widget.enter_focus();
        }

        if let Some(focused_slot) = self.focused_ids.last_mut() {
            *focused_slot = Some(widget_id);
        }

        Ok(())
    }

    /// Clears the focus of the highest layer
    pub fn exit_focus(&mut self) {
        let widget_id = *self.focused_ids.last().unwrap();

        if let Some(widget_id) = widget_id {
            if let Ok(dyn_widget) = self.get_widget_mut(widget_id) {
                if let Some(tf_widget) = downcast_widget!(mut dyn_widget, TextField) {
                    tf_widget.exit_focus();
                }
            }
        }

        if let Some(widget_id) = self.focused_ids.last_mut() {
            *widget_id = None;
        }
    }

    pub fn on_hover(&mut self, point: &Point2<f32>) {
        let highest_z_index = self.peek_z_index();

        for widget in self.widget_list.iter_mut()
            .filter(|widget| widget.z_index() == highest_z_index)
        {
            widget.on_hover(point);
        }
    }

    pub fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let highest_z_index = self.peek_z_index();

        for widget in self.widget_list.iter_mut()
            .filter(|widget| widget.z_index() == highest_z_index)
        {
            let ui_action = widget.on_click(point);
            if ui_action.is_some() {
                return ui_action;
            }
        }
        None
    }

    pub fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {
        let highest_z_index = self.peek_z_index();

        for widget in self.widget_list.iter_mut()
            .filter(|widget| widget.z_index() == highest_z_index)
        {
            widget.on_drag(original_pos, current_pos);
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> UIResult<()> {
        let highest_z_index = self.peek_z_index();

        if self.with_transparency && highest_z_index != 0 {
            // Draw the previous layer
            for widget in self.widget_list.iter_mut()
                .filter(|widget| widget.z_index() == highest_z_index - 1)
            {
                widget.draw(ctx)?;
            }

            // TODO: Get resolution from video-settings
            let mesh = graphics::Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(0.0, 0.0, 1920.0, 1080.0),
                *LAYER_TRANSPARENCY_BG_COLOR,
            )?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        for widget in self.widget_list.iter_mut()
            .filter(|widget| widget.z_index() == highest_z_index)
        {
            widget.draw(ctx)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{Chatbox, common::FontInfo};
    use crate::constants;
    use crate::ggez::{graphics::Scale, nalgebra::Vector2};

    fn create_dummy_widget_id(widget_id: WidgetID) -> Box<Chatbox> {
        let font_info = FontInfo {
            font: (),
            scale: Scale::uniform(1.0),
            char_dimensions: Vector2::<f32>::new(5.0, 5.0),
        };
        Box::new(Chatbox::new(widget_id, font_info, 5))
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
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);

        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_ok());

        for (i, w) in layer_info.widget_list.iter().enumerate() {
            assert_eq!(i, 0);
            assert_eq!(w.id(), WidgetID(0));
        }
    }

    #[test]
    fn test_add_widget_two_widget_share_the_same_id_at_same_z_depth() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_ok());

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_err());
    }

    #[test]
    fn test_add_widget_two_widget_share_the_same_id_at_different_z_depths() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_ok());

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), 1).is_err());
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let widget_id = WidgetID(0);
        let chatbox = Chatbox::new(widget_id, font_info, history_len);

        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_ok());
        let w = layer_info.get_widget_mut(widget_id).unwrap();
        assert_eq!(w.id(), WidgetID(0));
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists_not_at_default_depth() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let widget_id = WidgetID(0);
        let chatbox = Chatbox::new(widget_id, font_info, history_len);

        assert!(layer_info.add_widget(Box::new(chatbox), 1).is_ok());
        let w = layer_info.get_widget_mut(widget_id).unwrap();
        assert_eq!(w.id(), WidgetID(0));
    }

    #[test]
    fn test_get_widget_mut_widget_does_not_exist_list_is_empty() {
        let mut layer_info = Layering::new();

        assert!(layer_info.get_widget_mut(WidgetID(0)).is_err());
    }

    #[test]
    fn test_get_widget_mut_widget_does_not_exist_list_non_empty() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(1), font_info, history_len);

        assert!(layer_info.add_widget(Box::new(chatbox), 0).is_ok());
        assert!(layer_info.get_widget_mut(WidgetID(0)).is_err());
    }

    #[test]
    fn test_get_widget_mut_widget_is_a_pane() {
        let mut layer_info = Layering::new();
        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 100.0, 100.0));

        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());
        let w = layer_info.get_widget_mut(widget_id).unwrap();
        assert_eq!(w.id(), widget_id);
    }

    #[test]
    fn test_get_widget_mut_widget_is_within_a_pane() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let mut pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_size(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(pane.add(Box::new(chatbox)).is_ok());
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        let w = layer_info.get_widget_mut(chatbox_id).unwrap();
        assert_eq!(w.id(), WidgetID(1));
    }

    #[test]
    fn test_get_widget_mut_sub_widget_in_pane_but_id_does_not_exist() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let mut pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_size(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(pane.add(Box::new(chatbox)).is_ok());
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert!(layer_info.get_widget_mut(WidgetID(2)).is_err());
    }

    #[test]
    fn test_search_panes_for_widget_id_widget_not_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let mut pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_size(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(pane.add(Box::new(chatbox)).is_ok());
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert_eq!(layer_info.search_panes_for_widget_id(WidgetID(2)), None);
    }

    #[test]
    fn test_search_panes_for_widget_id_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let mut pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_size(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(pane.add(Box::new(chatbox)).is_ok());
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert_eq!(layer_info.search_panes_for_widget_id(chatbox_id), Some((0, 0)));
    }

    #[test]
    fn test_rebuild_id_cache_during_adding() {
        let mut layer_info = Layering::new();
        let limit = 10;

        for i in 0..limit {
            let widget_id = WidgetID(i);
            let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
            assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());
        }

        assert_eq!(layer_info.id_cache.len(), 10);
    }

    #[test]
    fn test_rebuild_id_cache_during_removing() {
        let mut layer_info = Layering::new();
        let limit = 10;

        for i in 0..limit {
            let widget_id = WidgetID(i);
            let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
            assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());
        }

        assert_eq!(layer_info.id_cache.len(), 10);

        for i in 0..limit {
            let widget_id = WidgetID(i);
            assert!(layer_info._remove_widget(widget_id).is_ok());
        }
    }

    #[test]
    fn test_layering_enter_focus_basic()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert!(layer_info.enter_focus(widget_id).is_ok());
        assert_eq!(layer_info.focused_widget_id(), Some(widget_id));
    }

    #[test]
    fn test_layering_enter_focus_widget_not_found()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert!(layer_info.enter_focus(WidgetID(1)).is_err());
        assert_eq!(layer_info.focused_widget_id(), None);
    }

    #[test]
    fn test_layering_exit_focus()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), 0).is_ok());

        assert!(layer_info.enter_focus(widget_id).is_ok());
        assert_eq!(layer_info.focused_widget_id(), Some(widget_id));

        layer_info.exit_focus();
        assert_eq!(layer_info.focused_widget_id(), None);
    }


    #[test]
    fn test_check_for_entry_widget_not_found() {
        let dummy_widget_id = WidgetID(0);
        let layer_info = Layering::new();

        assert!(!layer_info.check_for_entry(dummy_widget_id));
    }

    #[test]
    fn test_check_for_entry_widget_found() {
        let dummy_widget_id = WidgetID(0);
        let dummy_widget = create_dummy_widget_id(dummy_widget_id);
        let mut layer_info = Layering::new();

        let _result = layer_info.add_widget(dummy_widget, 0);
        assert!(layer_info.check_for_entry(dummy_widget_id));
    }

    #[test]
    fn test_check_for_entry_widget_not_found_list_non_empty() {
        let dummy_widget_id = WidgetID(0);
        let dummy_widget = create_dummy_widget_id(dummy_widget_id);
        let mut layer_info = Layering::new();

        let _result = layer_info.add_widget(dummy_widget, 0);
        assert!(layer_info.check_for_entry(dummy_widget_id));

        assert!(!layer_info.check_for_entry(WidgetID(1)));
    }
}