/*  Copyright 2019-2020 the Conwayste Developers.
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
use ggez::Context;

use id_tree::{*, InsertBehavior, RemoveBehavior};

use super::{
    BoxedWidget,
    widget::Widget,
    Pane,
    UIAction,
    UIError, UIResult,
    WidgetID
};

use crate::constants::{colors::*, LAYERING_NODE_CAPACITY, LAYERING_SWAP_CAPACITY};

/// Dummy Widget to act as a root node in the tree. Serves no other purpose.
#[derive(Debug)]
struct LayerRootNode;
impl LayerRootNode {
    fn new() -> BoxedWidget {
        Box::new(LayerRootNode{})
    }
}
impl Widget for LayerRootNode {
    fn id(&self) -> WidgetID {
        use std::usize::MAX;
        WidgetID(MAX)
    }
    fn z_index(&self) -> usize {
        use std::usize::MAX;
        MAX
    }
    fn rect(&self) -> Rect { Rect::new(0.0, 0.0, 0.0, 0.0)}
    fn position(&self) -> Point2<f32> {Point2::new(0.0, 0.0)}
    fn size(&self) -> (f32, f32) { (0.0, 0.0) }
    fn translate(&mut self, _dest: Vector2<f32>) { }
}

pub enum InsertModifier {
    AtCurrentLayer,
    AtNextLayer,
    ToNestedPane(WidgetID)  // WidgetID of destination Pane
}

pub struct Layering {
    pub with_transparency: bool,        // Determines if a transparent film is drawn in between two
                                        // adjacent layers
    pub widget_tree: Tree<BoxedWidget>,
    highest_z_order: usize,             // Number of layers allocated in the system + 1
    focused_node_id: Option<NodeId>,    // Currently active widget, one per z-order. If None, then
                                        // the layer is not focused on any particular widget.
}

/// A `Layering` is a container of one or more widgets or panes (hereby referred to as widgets),
/// ordered and drawn by by their `z_index`, to create the appearance of a depth for a given game
/// screen. Each screen must have only one layering to store the set of visible widgets.
///
/// Behind the scenes, a Layering uses a tree data-structure to organize widgets. Widgets can be
/// nested, where the container (such as a Pane) would be the parent node. Widgets collected by the
/// container are its children nodes.
///
/// A use case for layering could be a modal dialog, where a Pane (containing all of the dialog's
/// widgets) may be added to the layering at a higher z-order than what is currently present. When
/// modal dialog is dismissed, the Pane is removed from the layering by widget-id, and the previously
/// presented UI elements are then displayed unaffected.
///
/// Widgets declare their z-order by the `z_index` field. A z-order of zero corresponds to the
/// base (or zeroth) layer. Widgets with a `z_index` of one are drawn immediately above that layer,
/// and so on. Only the two highest z-orders are drawn to minimize screen clutter. This means if
/// three widgets -- each with a z-index of 0, 1, and 2, respectively -- are added to the `Layering`,
/// only widgets 1 and 2 are drawn in that respective order.
///
/// Layerings also support an optional transparency between two adjacent z-orders. If the
/// transparency option is enabled, `with_transparency == true`, then a transparent film spanning
/// the screen size is drawn in between layers `n-1` and `n`.
impl Layering {
    pub fn new() -> Self {
        Layering {
            widget_tree: TreeBuilder::new()
                            .with_node_capacity(LAYERING_NODE_CAPACITY)
                            .with_swap_capacity(LAYERING_SWAP_CAPACITY)
                            .with_root(Node::new(LayerRootNode::new()))
                            .build(),
            highest_z_order: 0,
            with_transparency: false,
            focused_node_id: None,
        }
    }

    /// Returns true if an entry with the provided WidgetID exists.
    fn check_for_entry(&self, widget_id: WidgetID) -> bool {
        // unwrap safe because a Layering should always have a dummy root-node
        let root_id = self.widget_tree.root_node_id().unwrap();
        let mut s = String::new();
        let _ = self.widget_tree.write_formatted(&mut s);
        println!("{}", s);
        self.widget_tree.traverse_level_order(&root_id).unwrap().find(|&node| node.data().id() == widget_id).is_some()
    }

    /// Collect all nodes in the tree belonging to the corresponding z_order
    fn collect_node_ids(&self, z_order: usize) -> Vec<NodeId> {
        let root_id = self.widget_tree.root_node_id().unwrap();
        self.widget_tree.traverse_level_order_ids(&root_id).unwrap()
            .filter(|node_id| {
                let node = self.widget_tree.get(node_id).unwrap();
                node.data().z_index() == z_order
            })
            .collect::<Vec<NodeId>>()
    }

    /// Retreives a mutable reference to a widget. This will search the widget tree for the
    /// provided widget-id.
    ///
    /// # Error
    /// A WidgetNotFound error will be returned if the widget-id is not found.
    /// does not exist in the internal list of widgets.
    pub fn get_widget_mut(&mut self, widget_id: WidgetID) -> UIResult<&mut BoxedWidget> {
        // unwrap safe because a Layering should always have a dummy root-node
        let root_id = self.widget_tree.root_node_id().unwrap();

        if let Some(node_id) = self.widget_tree.traverse_level_order_ids(&root_id).unwrap()
            .filter(|node_id| {
                let node = self.widget_tree.get(&node_id).unwrap();
                node.data().id() == widget_id
            })
            .next()
        {
            let node = self.widget_tree.get_mut(&node_id).unwrap();
            Ok(node.data_mut())
        }
        else {
            Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list", widget_id).to_owned()
            }))
        }
    }

    /// Add a widget to the layering, where the z-order is specified by the insert modifier.
    /// Widgets can be inserted at the current layer, at the next layer (one order higher), or nested
    /// to a widget-container (like a Pane). The widget's z-index is overridden by the destination
    /// layer's z-order.
    ///
    /// # Error
    /// A `WidgetIDCollision` error can be returned if the widget-id exists in this layering.
    /// An `InvalidAction` error can be returned if the widget addition operation fails.
    /// A `WidgetNotFound` error can be returned if the nested container's widget-id does not exist.
    // todo rename InsertModifier
    pub fn add_widget(&mut self, mut widget: BoxedWidget, modifier: InsertModifier) -> UIResult<()> {
        let widget_id = widget.id();
        if self.check_for_entry(widget_id) {
            return Err(Box::new(UIError::WidgetIDCollision {
                reason: format!("Widget with ID {:?} exists in layer's widget list.", widget_id)
            }));
        }

        let root_id = self.widget_tree.root_node_id().unwrap().clone();
        match modifier {
            InsertModifier::AtCurrentLayer => {
                widget.set_z_index(self.highest_z_order);
                self.widget_tree.insert(Node::new(widget), InsertBehavior::UnderNode(&root_id))
                    .or_else(|e| Err(Box::new(UIError::InvalidAction {
                        reason: format!("Error during insertion of {:?}, AtCurrentLayer({}): {}",
                            widget_id,
                            self.highest_z_order,
                            e)
                    })))?;
            }
            InsertModifier::AtNextLayer => {
                self.highest_z_order += 1;
                widget.set_z_index(self.highest_z_order);
                self.widget_tree.insert(Node::new(widget), InsertBehavior::UnderNode(&root_id))
                    .or_else(|e| Err(Box::new(UIError::InvalidAction {
                        reason: format!("Error during insertion of {:?}, AtCurrentLayer({}): {}",
                            widget_id,
                            self.highest_z_order,
                            e)
                    })))?;
            }
            InsertModifier::ToNestedPane(widget_id) => {
                if !self.check_for_entry(widget_id) {
                    return Err(Box::new(UIError::WidgetNotFound {
                        reason: format!("Pane with ID {:?} not found in tree. Cannot add {:?} to tree.", widget_id, widget.id())
                    }));
                }

                // First find the node_id that corresponds to the Pane we're adding to
                let mut node_id_found = None;
                for node_id in self.widget_tree.traverse_level_order_ids(&root_id).unwrap() {
                    let node = self.widget_tree.get(&node_id).unwrap();
                    let dyn_widget = node.data();
                    if let Some(pane) = downcast_widget!(dyn_widget, Pane) {
                        if pane.id() == widget_id {
                            // Prepare the widget for insertion at the Pane's layer, translated to
                            // an offset from the Pane's top-left corner
                            let point = pane.dimensions.point();
                            let vector = Vector2::new(point.x, point.y);
                            widget.translate(vector);
                            widget.set_z_index(pane.z_index());

                            node_id_found = Some(node_id);
                            break;
                        }
                    }
                }

                // Insert the node under the found node_id corresponding to the Pane
                if let Some(node_id) = node_id_found {
                    let inserting_widget_id = widget.id();
                    self.widget_tree.insert(Node::new(widget), InsertBehavior::UnderNode(&node_id))
                        .or_else(|e| Err(Box::new(UIError::InvalidAction {
                            reason: format!("Error during insertion of {:?}, ToNestedPane({:?}, layer={}): {}",
                                inserting_widget_id,
                                widget_id,
                                self.highest_z_order,
                                e)
                        })))?;
                } else {
                    return Err(Box::new(UIError::WidgetNotFound {
                        reason: format!(concat!("Pane with ID {:?} not found in tree but was checked",
                        " for entry. Cannot add {:?} to tree."), widget_id, widget.id())
                    }))
                }
            }
        }
        Ok(())
    }

    /// Removes a widget belonging to the layering. Will drop all child nodes if the target is a
    /// container-based widget.
    ///
    /// # Error
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist
    /// in the internal list of widgets.
    // Implemented API for future use. TODO: Remove comment once function is used
    #[allow(unused)]
    pub fn remove_widget(&mut self, widget_id: WidgetID) -> UIResult<()> {
        if !self.check_for_entry(widget_id) {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layer during removal", widget_id).to_owned()
            }));
        }

        let root_id = self.widget_tree.root_node_id().unwrap();
        for node_id in self.widget_tree.traverse_level_order_ids(&root_id).unwrap() {
            let node = self.widget_tree.get(&node_id).unwrap();
            let dyn_widget = node.data();
            if dyn_widget.id() == widget_id {
                self.widget_tree.remove_node(node_id, RemoveBehavior::DropChildren);
                break;
            }
        }

        Ok(())
    }

    /// Returns the WidgetID of the widget currently in-focus
    pub fn focused_widget_id(&self) -> Option<WidgetID> {
        self.focused_node_id.as_ref().map(|node_id| self.widget_tree.get(node_id).unwrap().data().id())
    }

    /// Notifies the layer that the provided WidgetID is to capture input events
    ///
    /// # Error
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist in
    /// the internal list of widgets.
    pub fn enter_focus(&mut self, widget_id: WidgetID) -> UIResult<()> {
        if !self.check_for_entry(widget_id) {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list during enter focus", widget_id)
            }));
        }

        let root_id = self.widget_tree.root_node_id().unwrap();
        for node_id in self.widget_tree.traverse_level_order_ids(&root_id).unwrap() {
            let node = self.widget_tree.get(&node_id).unwrap();
            let dyn_widget = node.data();
            if dyn_widget.id() == widget_id {
                // Will overwrite any previously focused widget. This is acceptable because the user
                // may be switching focuses, like from one textfield to another
                self.focused_node_id = Some(node_id);
                break;
            }
        }

        // Call the widget's handler to enter focus
        if let Some(node_id) = &self.focused_node_id {
            let node = self.widget_tree.get_mut(node_id).unwrap();
            let dyn_widget = node.data_mut();
            dyn_widget.enter_focus();
        }

        Ok(())
    }

    /// Clears the focus of the layering
    pub fn exit_focus(&mut self) {
        if let Some(widget_id) = self.focused_widget_id() {
            if let Ok(dyn_widget) = self.get_widget_mut(widget_id) {
                dyn_widget.exit_focus();
            }
        }

        #[allow(unused)]
        self.focused_node_id.take();
    }

    pub fn on_hover(&mut self, point: &Point2<f32>) {
        let node_ids = self.collect_node_ids(self.highest_z_order);

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
            widget.on_hover(point);
        }
    }

    pub fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let node_ids = self.collect_node_ids(self.highest_z_order);

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
            let ui_action = widget.on_click(point);
            if ui_action.is_some() {
                return ui_action;
            }
        }
        None
    }

    pub fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {
        let node_ids = self.collect_node_ids(self.highest_z_order);

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
            widget.on_drag(original_pos, current_pos);
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> UIResult<()> {
        if self.highest_z_order > 0 {
            // Draw the previous layer
            let node_ids = self.collect_node_ids(self.highest_z_order - 1);

            for node_id in node_ids {
                let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
                widget.draw(ctx)?;
            }

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
        }

        let node_ids = self.collect_node_ids(self.highest_z_order);

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
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

        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_ok());

        let widget_result = layer_info.get_widget_mut(WidgetID(0));
        assert!(widget_result.is_ok());
        let widget = widget_result.unwrap();
        assert_eq!(widget.id(), WidgetID(0));
    }

    #[test]
    fn test_add_widget_two_widget_share_the_same_id_at_same_z_depth() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_ok());

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_err());
    }

    #[test]
    fn test_add_widget_two_widget_share_the_same_id_at_different_z_depths() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_ok());

        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtNextLayer).is_err());
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let widget_id = WidgetID(0);
        let chatbox = Chatbox::new(widget_id, font_info, history_len);

        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_ok());
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

        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtNextLayer).is_ok());
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

        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::AtCurrentLayer).is_ok());
        assert!(layer_info.get_widget_mut(WidgetID(0)).is_err());
    }

    #[test]
    fn test_get_widget_mut_widget_is_a_pane() {
        let mut layer_info = Layering::new();
        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 100.0, 100.0));

        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());
        let w = layer_info.get_widget_mut(widget_id).unwrap();
        assert_eq!(w.id(), widget_id);
    }

    #[test]
    fn test_get_widget_mut_widget_is_within_a_pane() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::ToNestedPane(pane_id)).is_ok());

        let w = layer_info.get_widget_mut(chatbox_id).unwrap();
        assert_eq!(w.id(), WidgetID(1));
    }

    #[test]
    fn test_get_widget_mut_sub_widget_in_pane_but_id_does_not_exist() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::ToNestedPane(pane_id)).is_ok());

        assert!(layer_info.get_widget_mut(WidgetID(2)).is_err());
    }

    #[test]
    fn test_search_panes_for_widget_id_widget_not_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::ToNestedPane(pane_id)).is_ok());

        assert_eq!(layer_info.check_for_entry(WidgetID(2)), false);
    }

    #[test]
    fn test_search_panes_for_widget_id_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let pane_id = WidgetID(0);
        let chatbox_id = WidgetID(1);

        let pane = Pane::new(pane_id, *constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(chatbox_id, font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0
        ));

        assert!(size_update_result.is_ok());
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());
        assert!(layer_info.add_widget(Box::new(chatbox), InsertModifier::ToNestedPane(pane_id)).is_ok());

        assert_eq!(layer_info.check_for_entry(WidgetID(1)), true);
    }

    #[test]
    fn test_layering_enter_focus_basic()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());

        assert!(layer_info.enter_focus(widget_id).is_ok());
        assert_eq!(layer_info.focused_widget_id(), Some(widget_id));
    }

    #[test]
    fn test_layering_enter_focus_widget_not_found()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());

        assert!(layer_info.enter_focus(WidgetID(1)).is_err());
        assert_eq!(layer_info.focused_widget_id(), None);
    }

    #[test]
    fn test_layering_exit_focus()
    {
        let mut layer_info = Layering::new();

        let widget_id = WidgetID(0);
        let pane = Pane::new(widget_id, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert!(layer_info.add_widget(Box::new(pane), InsertModifier::AtCurrentLayer).is_ok());

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

        let _result = layer_info.add_widget(dummy_widget, InsertModifier::AtCurrentLayer);
        assert!(layer_info.check_for_entry(dummy_widget_id));
    }

    #[test]
    fn test_check_for_entry_widget_not_found_list_non_empty() {
        let dummy_widget_id = WidgetID(0);
        let dummy_widget = create_dummy_widget_id(dummy_widget_id);
        let mut layer_info = Layering::new();

        let _result = layer_info.add_widget(dummy_widget, InsertModifier::AtCurrentLayer);
        assert!(layer_info.check_for_entry(dummy_widget_id));

        assert!(!layer_info.check_for_entry(WidgetID(1)));
    }
}