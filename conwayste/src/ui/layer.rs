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

use ggez::graphics::{self, DrawMode, DrawParam, Rect};
use ggez::nalgebra::{Point2, Vector2};
use ggez::Context;

use id_tree::{
    InsertBehavior,
    RemoveBehavior,
    NodeId,
    Tree,
    TreeBuilder,
    Node
};

use super::{
    common::within_widget, widget::Widget, BoxedWidget, Pane, UIAction, UIError, UIResult,
};

use crate::constants::{colors::*, LAYERING_NODE_CAPACITY, LAYERING_SWAP_CAPACITY};

/// Dummy Widget to serve as a root node in the tree. Serves no other purpose.
#[derive(Debug)]
struct LayerRootNode {
    root_id: Option<NodeId>
}
impl LayerRootNode {
    fn new() -> BoxedWidget {
        Box::new(LayerRootNode {root_id: None})
    }
}

impl Widget for LayerRootNode {
    fn id(&self) -> Option<&NodeId> {
        self.root_id.as_ref()
    }

    fn set_id(&mut self, _new_id: NodeId) {
        // do nothing for now
    }

    fn z_index(&self) -> usize {
        use std::usize::MAX;
        MAX
    }
    fn rect(&self) -> Rect {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    }
    fn position(&self) -> Point2<f32> {
        Point2::new(0.0, 0.0)
    }
    fn size(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn translate(&mut self, _dest: Vector2<f32>) {}
}

#[allow(unused)]
pub enum InsertLocation<'a> {
    AtCurrentLayer, // Insertion will be made at whatever the top-most layer order is
    AtNextLayer,    // Insertion will increment the layer order, and insert
    ToNestedContainer(&'a NodeId), // Inserted as a child to the specified node in the tree
}

pub struct Layering {
    pub with_transparency: bool, // Determines if a transparent film is drawn in between two
    // adjacent layers
    widget_tree: Tree<BoxedWidget>, // Tree of widgets. Container-like widgets (think Panes)
    // will have children nodes which are the nested elements
    // (think Buttons) of the widget.
    highest_z_order: usize, // Number of layers allocated in the system + 1
    focused_node_id: Option<NodeId>, // Currently active widget, one per z-order. If None, then
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
/// widgets) may be added to the layering at a higher z-order than what is currently present. Here
/// the user would add to the tree using the `AtNextLayer` for the initial Pane, and
/// `ToNestedContainer(...)` for all child widgets. When modal dialog is dismissed, the Pane is
/// removed from the layering by node id using the remove API. Any previously presented UI prior
/// to the new layer will be displayed unaffected by the addition and removal of the Pane.
///
/// Widgets declare their z-order by the `z_index` field. A z-order of zero corresponds to the
/// base (or zeroth) layer. Widgets with a `z_index` of one are drawn immediately above that layer,
/// and so on. Only the two highest z-orders are drawn to minimize screen clutter. This means if
/// three widgets -- each with a z-index of 0, 1, and 2, respectively -- are added to the `Layering`,
/// only widgets 1 and 2 are drawn in that respective order. Widgets will have their `z_index`
/// updated based on the destination layer the widget ultimately ends up in.
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

    /// Returns true if an entry with the provided NodeId exists.
    fn widget_exists(&self, id: &NodeId) -> bool {
        let mut s = String::new();
        let _ = self.widget_tree.write_formatted(&mut s);
        debug!("{}", s);
        self.widget_tree.traverse_level_order(id).is_ok()
    }

    /// Collect all nodes in the tree belonging to the corresponding z_order
    fn collect_node_ids(&self, z_order: usize) -> Vec<NodeId> {
        let root_id = self.widget_tree.root_node_id().unwrap();
        self.widget_tree
            .traverse_level_order_ids(&root_id)
            .unwrap()
            .filter(|node_id| {
                let node = self.widget_tree.get(node_id).unwrap();
                node.data().z_index() == z_order
            })
            .collect::<Vec<NodeId>>()
    }

    /// Retreives a mutable reference to a widget. This will search the widget tree for the
    /// provided node id.
    ///
    /// # Errors
    ///
    /// A WidgetNotFound error will be returned if the node id is not found.
    /// does not exist in the internal list of widgets.
    pub fn get_widget_mut(&mut self, id: &NodeId) -> UIResult<&mut BoxedWidget> {
        if let Ok(node) = self.widget_tree.get_mut(id) {
            Ok(node.data_mut())
        } else {
            Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list", id).to_owned(),
            }))
        }
    }

    /// Add a widget to the layering, where the z-order is specified by the insert modifier.
    /// Widgets can be inserted at the current layer, at the next layer (one order higher), or nested
    /// to a widget-container (like a Pane). The widget's z-index is overridden by the destination
    /// layer's z-order. `set_id` is called on the widget after insertion to update the widget's id.
    ///
    /// # Return
    /// Returns a unique node identifier assigned to the successfully inserted widget.
    ///
    /// # Errors
    ///
    /// A `NodeIDCollision` error can be returned if the node id exists in this layering.
    /// An `InvalidAction` error can be returned if the widget addition operation fails.
    /// A `WidgetNotFound` error can be returned if the nested container's node id does not exist.
    pub fn add_widget(
        &mut self,
        mut widget: BoxedWidget,
        modifier: InsertLocation,
    ) -> UIResult<NodeId> {
        // Check that we aren't inserting a widget into the tree that already exists
         if let Some(id) = widget.id() {
            return Err(Box::new(UIError::NodeIDCollision {
                reason: format!("Widget with ID {:?} exists was assigned an ID already.", id),
            }));
        }

        // Unwrap safe because our tree will always have a dummy root node
        let root_id = self.widget_tree.root_node_id().unwrap().clone();
        let inserted_node_id;
        match modifier {
            InsertLocation::AtCurrentLayer => {
                widget.set_z_index(self.highest_z_order);
                inserted_node_id = self.widget_tree
                    .insert(Node::new(widget), InsertBehavior::UnderNode(&root_id))
                    .or_else(|e| {
                        Err(Box::new(UIError::InvalidAction {
                            reason: format!(
                                "Error during insertion AtCurrentLayer({}): {}",
                                self.highest_z_order, e
                            ),
                        }))
                    })?;
            }
            InsertLocation::AtNextLayer => {
                self.highest_z_order += 1;
                widget.set_z_index(self.highest_z_order);
                inserted_node_id = self.widget_tree
                    .insert(Node::new(widget), InsertBehavior::UnderNode(&root_id))
                    .or_else(|e| {
                        Err(Box::new(UIError::InvalidAction {
                            reason: format!(
                                "Error during insertion AtNextLayer({}): {}",
                                self.highest_z_order, e
                            ),
                        }))
                    })?;
            }
            InsertLocation::ToNestedContainer(parent_id) => {
                if !self.widget_exists(parent_id) {
                    return Err(Box::new(UIError::WidgetNotFound {
                        reason: format!(
                            "Parent Container with NodeId {:?} not found in tree. Cannot nest {:?}.",
                            parent_id,
                            widget
                        ),
                    }));
                }

                // First find the node_id that corresponds to the container we're adding to
                let node = self.widget_tree.get(&parent_id).unwrap();
                let dyn_widget = node.data();
                if let Some(pane) = downcast_widget!(dyn_widget, Pane) {
                    // Prepare the widget for insertion at the Pane's layer, translated to
                    // an offset from the Pane's top-left corner
                    let point = pane.dimensions.point();
                    let vector = Vector2::new(point.x, point.y);
                    widget.translate(vector);
                    widget.set_z_index(pane.z_index());
                }

                // Insert the node under the found node_id corresponding to the Pane
                inserted_node_id = self.widget_tree.insert(Node::new(widget), InsertBehavior::UnderNode(&parent_id))
                    .or_else(|e| Err(Box::new(UIError::InvalidAction {
                        reason: format!("Error during insertion, ToNestedContainer({:?}, layer={}): {}",
                            parent_id,
                            self.highest_z_order,
                            e)
                })))?;
            }
        }

        // Unwrap *should* be safe because otherwise we would have bailed prior to insertion
        let node = self.widget_tree.get_mut(&inserted_node_id).unwrap();
        node.data_mut().set_id(inserted_node_id.clone());

        Ok(inserted_node_id)
    }

    /// Removes a widget belonging to the layering. Will drop all child nodes if the target is a
    /// container-based widget.
    ///
    /// # Errors
    ///
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist
    /// in the internal list of widgets.
    // Implemented API for future use. TODO: Remove comment once function is used
    #[allow(unused)]
    pub fn remove_widget(&mut self, id: NodeId) -> UIResult<()> {
        if !self.widget_exists(&id) {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layer during removal", id).to_owned(),
            }));
        }

        self.widget_tree.remove_node(id, RemoveBehavior::DropChildren);

        // Determine if the highest z-order changes due to the widget removal by checking no other
        // widgets are present at that z_order
        while (self.highest_z_order != 0 &&
            self.collect_node_ids(self.highest_z_order).is_empty()) {
            self.highest_z_order -= 1;
        }

        Ok(())
    }

    /// Returns the NodeId of the widget currently in-focus
    pub fn focused_widget_id(&self) -> Option<NodeId> {
        self.focused_node_id.clone()
    }

    /// Notifies the layer that the provided NodeId is to capture input events
    ///
    /// # Errors
    ///
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist in
    /// the internal list of widgets.
    pub fn enter_focus(&mut self, id: &NodeId) -> UIResult<()> {
        if !self.widget_exists(id) {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!("{:?} not found in layering's widget list during enter focus", id),
            }));
        }

        // Will overwrite any previously focused widget. This is acceptable because the user
        // may be switching focuses, like from one textfield to another.
        self.focused_node_id = Some((*id).clone());

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
        if let Some(id) = self.focused_widget_id() {
            if let Ok(dyn_widget) = self.get_widget_mut(&id) {
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

    //TODO: this doesn't let container widgets control whether or how their child widgets get the
    //events. Consider only collecting a specific Node's childrens' NodeIds.
    pub fn on_click(&mut self, point: &Point2<f32>) -> Option<UIAction> {
        let node_ids = self.collect_node_ids(self.highest_z_order);

        // Due to the way `collect_node_ids()` traverses the entire list, all children nodes will be
        // collected for a parent node as they should be at the same z-order.
        // TODO: After UIContext lands, reevaluate how a child's on_click Handled will propogate up.

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
            if within_widget(point, &widget.rect()) {
                let ui_action = widget.on_click(point);
                if ui_action.is_some() {
                    return ui_action;
                }
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
    use super::super::{common::FontInfo, Chatbox};
    use super::*;
    use crate::constants;
    use crate::ggez::{graphics::Scale, nalgebra::Vector2};

    fn create_dummy_font() -> FontInfo {
        FontInfo {
            font: (),                   //dummy font because we can't create a real Font without ggez
            scale: Scale::uniform(1.0), // Does not matter
            char_dimensions: Vector2::<f32>::new(5.0, 5.0), // any positive values will do
        }
    }

    #[test]
    fn test_add_widget_to_layer_basic() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(font_info, history_len);

        let id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer);

        assert!(id.is_ok());
        let id = id.unwrap();

        let widget_result = layer_info.get_widget_mut(&id);
        assert!(widget_result.is_ok());
        let widget = widget_result.unwrap();
        assert_eq!(widget.id(), Some(&id));
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(font_info, history_len);

        let id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer);
        assert!(id.is_ok());
        let id = id.unwrap();

        let w = layer_info.get_widget_mut(&id).unwrap();
        assert_eq!(w.id(), Some(&id));
    }

    #[test]
    fn test_get_widget_mut_one_widget_exists_not_at_default_depth() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let chatbox = Chatbox::new(font_info, history_len);

        let id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::AtNextLayer).unwrap();

        let w = layer_info.get_widget_mut(&id).unwrap();
        assert_eq!(w.id(), Some(&id));
    }

    #[test]
    fn test_new_widget_tree_has_root_node_only() {
        let layer_info = Layering::new();

        let id = layer_info.widget_tree.root_node_id();
        assert!(id.is_some());
        let id = id.unwrap();
        let mut children = layer_info.widget_tree.children_ids(&id).unwrap();
        assert_eq!(children.next(), None);
    }

    #[test]
    fn test_get_widget_mut_widget_does_not_exist_list() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(font_info, history_len);

        // Add the widget to generate a NodeId
        let id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer).unwrap();

        // Remove the widget and perform a check of the ID
        let removal = layer_info.remove_widget(id.clone());
        assert_eq!(removal.is_ok(), true);
        assert!(layer_info.get_widget_mut(&id).is_err());
    }

    #[test]
    fn test_get_widget_mut_widget_is_a_pane() {
        let mut layer_info = Layering::new();
        let pane = Pane::new(Rect::new(0.0, 0.0, 100.0, 100.0));

        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        let w = layer_info.get_widget_mut(&pane_id).unwrap();
        assert_eq!(w.id(), Some(&pane_id));
    }

    #[test]
    fn test_get_widget_mut_widget_is_within_a_pane() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();

        let pane = Pane::new(*constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0,
        ));

        assert!(size_update_result.is_ok());
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();
        let chatbox_id = layer_info
            .add_widget(
                Box::new(chatbox),
                InsertLocation::ToNestedContainer(&pane_id)
            ).unwrap();

        let w = layer_info.get_widget_mut(&chatbox_id).unwrap();
        assert_eq!(w.id(), Some(&chatbox_id));
    }

    #[test]
    fn test_search_panes_for_widget_id_widget_not_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();

        let pane = Pane::new(*constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0,
        ));

        assert!(size_update_result.is_ok());
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();
        let chatbox_id = layer_info
            .add_widget(
                Box::new(chatbox),
                InsertLocation::ToNestedContainer(&pane_id)
            ).unwrap();

        let removal = layer_info.remove_widget(chatbox_id.clone());
        assert_eq!(removal.is_ok(), true);

        assert!(layer_info.get_widget_mut(&chatbox_id).is_err());
    }

    #[test]
    fn test_search_panes_for_widget_id_found() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();

        let pane = Pane::new(*constants::DEFAULT_CHATBOX_RECT);
        let history_len = 5;
        let mut chatbox = Chatbox::new(font_info, history_len);

        let size_update_result = chatbox.set_rect(Rect::new(
            0.0,
            0.0,
            constants::DEFAULT_CHATBOX_RECT.w,
            constants::DEFAULT_CHATBOX_RECT.h - 20.0,
        ));

        assert!(size_update_result.is_ok());
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();
        let chatbox_id = layer_info
            .add_widget(
                Box::new(chatbox),
                InsertLocation::ToNestedContainer(&pane_id)
            ).unwrap();

        assert_eq!(layer_info.widget_exists(&pane_id), true);
        assert_eq!(layer_info.widget_exists(&chatbox_id), true);
    }

    #[test]
    fn test_layering_enter_focus_basic() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        assert!(layer_info.enter_focus(&pane_id).is_ok());
        assert_eq!(layer_info.focused_widget_id(), Some(pane_id));
    }

    #[test]
    fn test_layering_enter_focus_widget_not_found() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let _pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        let pane2 = Pane::new(Rect::new(10.0, 10.0, 1.0, 1.0));
        let pane_id2 = layer_info
            .add_widget(Box::new(pane2), InsertLocation::AtCurrentLayer).unwrap();

        let removal = layer_info.remove_widget(pane_id2.clone());
        assert_eq!(removal.is_ok(), true);

        assert!(layer_info.enter_focus(&pane_id2).is_err());
        assert_eq!(layer_info.focused_widget_id(), None);
    }

    #[test]
    fn test_layering_exit_focus() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        assert!(layer_info.enter_focus(&pane_id).is_ok());
        assert_eq!(layer_info.focused_widget_id(), Some(pane_id));

        layer_info.exit_focus();
        assert_eq!(layer_info.focused_widget_id(), None);
    }

    #[test]
    fn test_widget_exists_widget_found() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();
        assert_eq!(layer_info.widget_exists(&pane_id), true);
    }

    #[test]
    fn test_widget_exists_widget_not_found_list_non_empty() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();
        assert_eq!(layer_info.widget_exists(&pane_id), true);

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);
        assert_eq!(layer_info.widget_exists(&pane_id), false);
    }

    #[test]
    fn test_remove_widget_successfully() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);
    }

    #[test]
    fn test_remove_widget_twice_fails() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer).unwrap();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), false);
    }
}
