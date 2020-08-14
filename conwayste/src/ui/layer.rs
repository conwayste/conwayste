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

use std::error::Error;

use std::collections::HashSet;

use ggez::graphics::{self, DrawMode, DrawParam, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::nalgebra::{Point2, Vector2};
use ggez::Context;

use id_tree::{InsertBehavior, Node, NodeId, RemoveBehavior, Tree, TreeBuilder};

use super::{
    common::within_widget,
    context::{self, KeyCodeOrChar},
    focus::{CycleType, FocusCycle},
    treeview,
    widget::Widget,
    BoxedWidget, Pane, UIError, UIResult,
};

use crate::config;
use crate::constants::{colors::*, LAYERING_NODE_CAPACITY, LAYERING_SWAP_CAPACITY};
use crate::Screen; // for screen stack

/// Dummy Widget to serve as a root node in the tree. Serves no other purpose.
#[derive(Debug)]
struct LayerRootNode {
    root_id: Option<NodeId>,
}
impl LayerRootNode {
    fn new() -> BoxedWidget {
        Box::new(LayerRootNode { root_id: None })
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
    fn translate(&mut self, _dest: Vector2<f32>) {
    }
}

#[allow(unused)]
#[derive(Eq, PartialEq)]
pub enum InsertLocation<'a> {
    AtCurrentLayer,                // Insertion will be made at whatever the top-most layer order is
    AtNextLayer,                   // Insertion will increment the layer order, and insert
    ToNestedContainer(&'a NodeId), // Inserted as a child to the specified node in the tree
}

pub struct Layering {
    pub with_transparency: bool, // Determines if a transparent film is drawn in between two
    // adjacent layers
    widget_tree:           Tree<BoxedWidget>, // Tree of widgets. Container-like widgets (think Panes)
    // will have children nodes which are the nested elements
    // (think Buttons) of the widget.
    removed_node_ids:      HashSet<NodeId>, // Set of all node-ids that have been removed from the Tree
    highest_z_order:       usize,           // Number of layers allocated in the system + 1
    focus_cycles:          Vec<FocusCycle>, // For each layer, a "FocusCycle" keeping track of which widgets
                                            // can be tabbed through to get focus, in which order, and which
                                            // widget of these (if any) has focus.
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
            widget_tree:       TreeBuilder::new()
                .with_node_capacity(LAYERING_NODE_CAPACITY)
                .with_swap_capacity(LAYERING_SWAP_CAPACITY)
                .with_root(Node::new(LayerRootNode::new()))
                .build(),
            removed_node_ids:  HashSet::new(),
            highest_z_order:   0,
            with_transparency: false,
            focus_cycles:      vec![FocusCycle::new(CycleType::Circular)], // empty focus cycle for z_order 0
        }
    }

    /// Returns true if an entry with the provided NodeId exists.
    fn widget_exists(&self, id: &NodeId) -> bool {
        self.widget_tree.get(id).is_ok()
    }

    pub fn debug_display_widget_tree(&self) {
        let mut s = String::new();
        let _ = self.widget_tree.write_formatted(&mut s);
        debug!("{}", s);
    }

    /// Collect all nodes in the tree belonging to the corresponding z_order. The node IDs are
    /// collected in level order, meaning the root appears before its children, and its children
    /// before their children, and so on.
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

    /// Retrieves a mutable reference to a widget. This will search the widget tree for the
    /// provided node id.
    ///
    /// # Errors
    ///
    /// A WidgetNotFound error will be returned if the node id is not found.
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
    ///
    /// Returns a unique node identifier assigned to the successfully inserted widget.
    ///
    /// # Errors
    ///
    /// A `NodeIDCollision` error can be returned if the node id exists in this layering.
    /// An `InvalidAction` error can be returned if the widget addition operation fails.
    /// A `WidgetNotFound` error can be returned if the nested container's node id does not exist.
    pub fn add_widget(&mut self, mut widget: BoxedWidget, modifier: InsertLocation) -> UIResult<NodeId> {
        // Check that we aren't inserting a widget into the tree that already exists
        if let Some(id) = widget.id() {
            return Err(Box::new(UIError::NodeIDCollision {
                reason: format!("Attempted to insert widget with previously assigned ID {:?}.", id),
            }));
        }
        let widget_accepts_keyboard_events = widget.accepts_keyboard_events();

        // Unwrap safe because our tree will always have a dummy root node
        let root_id = self.widget_tree.root_node_id().unwrap().clone();
        let inserted_node_id;
        let mut added_to_pane_focus_cycle = false;
        if modifier == InsertLocation::AtNextLayer {
            self.highest_z_order += 1;
            self.focus_cycles.push(FocusCycle::new(CycleType::Circular));
        }
        match modifier {
            InsertLocation::AtCurrentLayer | InsertLocation::AtNextLayer => {
                widget.set_z_index(self.highest_z_order);
                inserted_node_id = self
                    .widget_tree
                    .insert(Node::new(widget), InsertBehavior::UnderNode(&root_id))
                    .or_else(|e| {
                        Err(Box::new(UIError::InvalidAction {
                            reason: format!(
                                "Error during insertion {}, z_order={}: {}",
                                stringify!(modifier),
                                self.highest_z_order,
                                e,
                            ),
                        }))
                    })?;
            }
            InsertLocation::ToNestedContainer(parent_id) => {
                if !self.widget_exists(parent_id) {
                    return Err(Box::new(UIError::WidgetNotFound {
                        reason: format!(
                            "Parent Container with NodeId {:?} not found in tree. Cannot nest {:?}.",
                            parent_id, widget
                        ),
                    }));
                }

                // First find the node_id that corresponds to the container we're adding to
                let node = self.widget_tree.get(&parent_id).unwrap();
                let parent_dyn_widget = node.data();
                if let Some(pane) = downcast_widget!(parent_dyn_widget, Pane) {
                    // Prepare the widget for insertion at the Pane's layer, translated to
                    // an offset from the Pane's top-left corner
                    let point = pane.dimensions.point();
                    let vector = Vector2::new(point.x, point.y);
                    widget.translate(vector);
                    widget.set_z_index(pane.z_index());
                }

                // Insert the node under the found node_id corresponding to the Pane
                inserted_node_id = self
                    .widget_tree
                    .insert(Node::new(widget), InsertBehavior::UnderNode(&parent_id))
                    .or_else(|e| {
                        Err(Box::new(UIError::InvalidAction {
                            reason: format!(
                                "Error during insertion, ToNestedContainer({:?}, z_order={}): {}",
                                parent_id, self.highest_z_order, e
                            ),
                        }))
                    })?;
                let parent_dyn_widget = self.widget_tree.get_mut(&parent_id).unwrap().data_mut();
                if let Some(pane) = downcast_widget_mut!(parent_dyn_widget, Pane) {
                    // notify the Pane that we added a widget to it (used for keyboard focus)
                    if widget_accepts_keyboard_events {
                        pane.add_widget(inserted_node_id.clone());
                        added_to_pane_focus_cycle = true;
                    }
                }
            }
        }

        // Unwrap *should* be safe because otherwise we would have bailed prior to insertion
        let node = self.widget_tree.get_mut(&inserted_node_id).unwrap();

        // If the widget we just inserted can accept keyboard events, add it to the focus cycle.
        if !added_to_pane_focus_cycle && widget_accepts_keyboard_events {
            self.focus_cycles[self.highest_z_order].push(inserted_node_id.clone());
        }
        node.data_mut().set_id(inserted_node_id.clone());

        // Note the behavior if id_tree (somehow) reused an ID
        if self.removed_node_ids.contains(&inserted_node_id) {
            warn!(
                "NodeId {:?} found in removed-hashset during widget insertion. Possible reusage!",
                inserted_node_id
            );
        }

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

        // call remove_widget on its containing Pane, if any
        let pane_id = self.widget_tree.ancestor_ids(&id).unwrap().nth(0); // unwrap OK (id is valid)
        if let Some(pane_id) = pane_id {
            let pane_id = pane_id.clone();
            let parent_dyn_widget = self.widget_tree.get_mut(&pane_id).unwrap().data_mut();
            if let Some(pane) = downcast_widget_mut!(parent_dyn_widget, Pane) {
                // notify the Pane that we removed a widget from it (used for keyboard focus)
                pane.remove_widget(&id);
            }
        }

        // Insert the node's children ids to the removed hash-set
        if let Ok(children_ids) = self.widget_tree.children_ids(&id) {
            // collect nodes to bypass issue with double borrow on ChildrenIds iterator
            for node_id_ref in children_ids {
                self.removed_node_ids.insert((*node_id_ref).clone());
            }
        }

        // Finally check the node itself
        // clone is okay because the HashSet is intended to keep track of all removed widget ids
        // result not checked as this is reported during widget insertion
        self.removed_node_ids.insert(id.clone());

        // Remove from focus cycle
        self.focus_cycles[self.highest_z_order].remove(&id);

        // clone is okay because it is required
        self.widget_tree
            .remove_node(id.clone(), RemoveBehavior::DropChildren)
            .or_else(|e| {
                return Err(Box::new(UIError::InvalidAction {
                    // clone is okay for error reporting
                    reason: format!("NodeIDError occurred during removal of {:?}: {:?}", id.clone(), e),
                }));
            })?;

        // Determine if the highest z-order changes due to the widget removal by checking no other
        // widgets are present at that z_order
        while self.highest_z_order != 0 && self.collect_node_ids(self.highest_z_order).is_empty() {
            self.highest_z_order -= 1;
            self.focus_cycles.pop();
        }

        Ok(())
    }

    /// Returns the NodeId of the widget currently in-focus
    #[allow(unused)]
    pub fn focused_widget_id(&self) -> Option<&NodeId> {
        self.focus_cycles[self.highest_z_order].focused_widget_id()
    }

    /// Notifies the layer that the provided NodeId is to capture input events
    ///
    /// # Errors
    ///
    /// A WidgetNotFound error can be returned if a widget with the `widget_id` does not exist in
    /// the internal list of widgets.
    pub fn enter_focus(
        &mut self,
        ggez_context: &mut ggez::Context,
        cfg: &mut config::Config,
        screen_stack: &mut Vec<Screen>,
        game_in_progress: bool,
        id: &NodeId,
    ) -> UIResult<()> {
        let focus_cycle = &mut self.focus_cycles[self.highest_z_order];
        if focus_cycle.find(id).is_none() {
            return Err(Box::new(UIError::WidgetNotFound {
                reason: format!(
                    "{:?} either not found in layering's widget list or can't receive focus",
                    id
                ),
            }));
        }

        // Will overwrite any previously focused widget. This is acceptable because the user
        // may be switching focuses, like from one textfield to another.
        let (was_successful, old_focused_widget) = focus_cycle.set_focused(id);
        if !was_successful {
            // if we failed to set the focus, don't call any focus change handlers
            // (I don't think this is possible?)
            return Ok(());
        }

        let widget_view = treeview::TreeView::new(&mut self.widget_tree);

        let mut uictx = context::UIContext::new(ggez_context, cfg, widget_view, screen_stack, game_in_progress);

        let mut needs_gain_focus = true;
        if let Some(old_focused_widget) = old_focused_widget {
            if old_focused_widget != *id {
                // old ID loses focus
                Layering::emit_focus_change(context::EventType::LoseFocus, &mut uictx, &old_focused_widget).map_err(
                    |e| UIError::InvalidAction {
                        reason: format!("{:?}", e),
                    },
                )?;
            } else {
                needs_gain_focus = false;
            }
        }
        if needs_gain_focus {
            // new ID gains focus
            Layering::emit_focus_change(context::EventType::GainFocus, &mut uictx, id).map_err(|e| {
                UIError::InvalidAction {
                    reason: format!("{:?}", e),
                }
            })?;
        }

        Ok(())
    }

    /* TODO: re-implement using events
    pub fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {
        let node_ids = self.collect_node_ids(self.highest_z_order);

        for node_id in node_ids {
            let widget = self.widget_tree.get_mut(&node_id).unwrap().data_mut();
            widget.on_drag(original_pos, current_pos);
        }
    }
    */

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

    /// Emit an event on this Layering. Note that this is not part of impl EmitEvent for Layering!
    /// Layering does not implement this trait! It is this way to avoid mutably borrowing things
    /// more than once.
    pub fn emit(
        &mut self,
        event: &context::Event,
        ggez_context: &mut ggez::Context,
        cfg: &mut config::Config,
        screen_stack: &mut Vec<Screen>,
        game_in_progress: bool,
    ) -> Result<(), Box<dyn Error>> {
        let widget_view = treeview::TreeView::new(&mut self.widget_tree);
        let mut uictx = context::UIContext::new(ggez_context, cfg, widget_view, screen_stack, game_in_progress);
        if event.what == context::EventType::Update || event.what == context::EventType::MouseMove {
            Layering::broadcast_event(event, &mut uictx)
        } else if event.is_mouse_event() {
            Layering::emit_mouse_event(event, &mut uictx, &mut self.focus_cycles[self.highest_z_order])
        } else if event.is_key_event() {
            Layering::handle_keyboard_event(event, &mut uictx, &mut self.focus_cycles[self.highest_z_order])
        } else {
            warn!("Don't know how to handle event type {:?}", event.what); // nothing to do if this is not a key or a mouse event
            Ok(())
        }
    }

    fn broadcast_event(event: &context::Event, uictx: &mut context::UIContext) -> Result<(), Box<dyn Error>> {
        for child_id in uictx.widget_view.children_ids() {
            // Get a mutable reference to a BoxedWidget, as well as a UIContext with a view on the
            // widgets in the tree under this widget.
            let (widget_ref, mut subuictx) = uictx.derive(&child_id).unwrap(); // unwrap OK b/c NodeId valid & in view

            if let Some(emittable) = widget_ref.as_emit_event() {
                emittable.emit(event, &mut subuictx)?;
                let pane_events = subuictx.collect_child_events();
                if pane_events.len() != 0 {
                    warn!(
                        "[Layering] expected no {:?} child events to be collected from child widget; got {:?}",
                        event.what, pane_events
                    );
                }
            }
        }
        Ok(())
    }

    fn handle_keyboard_event(
        event: &context::Event,
        uictx: &mut context::UIContext,
        focus_cycle: &mut FocusCycle,
    ) -> Result<(), Box<dyn Error>> {
        let key = event.key.ok_or_else(|| -> Box<dyn Error> {
            format!("layering event of type {:?} has no key", event.what).into()
        })?;

        if key == KeyCodeOrChar::KeyCode(KeyCode::Tab) {
            // special key press logic to handle focus changes

            let opt_child_id = focus_cycle.focused_widget_id().map(|child_id_ref| child_id_ref.clone());
            let opt_widget = opt_child_id
                .as_ref()
                .map(|child_id| uictx.widget_view.get(child_id).unwrap().data());
            if opt_child_id.is_some() && opt_widget.unwrap().downcast_ref::<Pane>().is_some() {
                let child_id = opt_child_id.unwrap();
                let pane_events = Layering::emit_keyboard_event(event, uictx, &child_id)?;

                // check if the Pane's focus dropped of the end of its open-ended focus "cycle"
                Layering::handle_reflexive_event(key, focus_cycle, uictx, &pane_events[..], event.shift_pressed)?;
            } else {
                if event.shift_pressed {
                    focus_cycle.focus_previous();
                } else {
                    focus_cycle.focus_next();
                }

                // Only send gain/lose events if the newly focused widget is different from the
                // previously focused widget.
                if focus_cycle.focused_widget_id() != opt_child_id.as_ref() {
                    // send a GainFocus event to the newly focused widget (if any)
                    if let Some(newly_focused_id) = focus_cycle.focused_widget_id() {
                        Layering::emit_focus_change(context::EventType::GainFocus, uictx, newly_focused_id)?;
                    }

                    // send a LoseFocus event to the previously focused widget (if any)
                    if let Some(newly_focused_id) = opt_child_id {
                        Layering::emit_focus_change(context::EventType::LoseFocus, uictx, &newly_focused_id)?;
                    }
                }
            }

            Ok(())
        } else {
            // regular key press logic (no focus changes)
            let focused_id = focus_cycle.focused_widget_id();
            println!("{:?}", focused_id);
            if let Some(id) = focused_id {
                let id = id.clone();
                let pane_events = Layering::emit_keyboard_event(event, uictx, &id)?;
                Layering::handle_reflexive_event(key, focus_cycle, uictx, &pane_events[..], false)?;
            } else {
                // XXX pick a better name because I can't handle this 😂
                Layering::handle_unhandled_keyboard_events(event, uictx)?;
            }
            Ok(())
        }
    }

    fn handle_reflexive_event(
        key: KeyCodeOrChar,
        focus_cycle: &mut FocusCycle,
        uictx: &mut context::UIContext,
        events: &[context::Event],
        shift_pressed: bool,
    ) -> Result<(), Box<dyn Error>> {
        for event in events {
            // ignore all event types except this one for now
            if event.what == context::EventType::ChildReleasedFocus {
                // Special keys
                if key == KeyCodeOrChar::KeyCode(KeyCode::Tab) {
                    if shift_pressed {
                        focus_cycle.focus_previous();
                    } else {
                        focus_cycle.focus_next();
                    }
                } else if key == KeyCodeOrChar::KeyCode(KeyCode::Escape) {
                    focus_cycle.clear_focus();
                }
                // send a GainFocus event to the newly focused widget (if any)
                if let Some(newly_focused_id) = focus_cycle.focused_widget_id() {
                    let newly_focused_id = newly_focused_id.clone();
                    Layering::emit_focus_change(context::EventType::GainFocus, uictx, &newly_focused_id)?;
                }
                break;
            }
        }
        Ok(())
    }

    fn emit_keyboard_event(
        event: &context::Event,
        uictx: &mut context::UIContext,
        focused_id: &NodeId,
    ) -> Result<Vec<context::Event>, Box<dyn Error>> {
        let mut unhandled_event = false;
        let mut child_events = vec![];
        let (widget_ref, mut subuictx) = uictx.derive(&focused_id).unwrap(); // unwrap OK b/c NodeId valid & in view
        if let Some(emittable) = widget_ref.as_emit_event() {
            emittable.emit(event, &mut subuictx).map(|handled| {
                println!("Handled? : {:?}", handled);
                // Nothing picked up the key event, send it to the Layer itself
                unhandled_event = handled == context::Handled::NotHandled;
                let events = subuictx.collect_child_events();
                child_events.extend_from_slice(&events[..]);
            })?;
        } else {
            // We probably won't ever get here due to the FocusCycle only holding widgets that can
            // receive keyboard events.
            warn!("nothing to emit on; widget is not an EmitEvent");
        }

        drop(subuictx);

        if unhandled_event {}

        Ok(child_events)
    }

    fn emit_focus_change(
        what: context::EventType,
        uictx: &mut context::UIContext,
        focused_id: &NodeId,
    ) -> Result<(), Box<dyn Error>> {
        if what != context::EventType::GainFocus && what != context::EventType::LoseFocus {
            return Err(format!("Unexpected event type passed to Pane::emit_focus_change: {:?}", what).into());
        }
        let (widget_ref, mut subuictx) = uictx.derive(&focused_id).unwrap(); // unwrap OK b/c NodeId valid & in view
        if let Some(emittable) = widget_ref.as_emit_event() {
            let event = context::Event::new_gain_or_lose_focus(what);
            emittable.emit(&event, &mut subuictx)?;
            let pane_events = subuictx.collect_child_events();
            if pane_events.len() != 0 {
                warn!(
                    "[Layering] emit focus chg: expected no child events to be collected from Pane; got {:?}",
                    pane_events
                );
            }
            return Ok(());
        } else {
            // We probably won't ever get here due to the FocusCycle only holding widgets that can
            // receive keyboard events.
            debug!("nothing to emit on; widget is not an EmitEvent");
        }
        Ok(())
    }

    fn emit_mouse_event(
        event: &context::Event,
        uictx: &mut context::UIContext,
        focus_cycle: &mut FocusCycle,
    ) -> Result<(), Box<dyn Error>> {
        let point = event
            .point
            .as_ref()
            .ok_or_else(|| -> Box<dyn Error> { format!("event of type {:?} has no point", event.what).into() })?;

        let mut child_events = vec![];

        for child_id in uictx.widget_view.children_ids() {
            // Get a mutable reference to a BoxedWidget, as well as a UIContext with a view on the
            // widgets in the tree under this widget.
            let (widget_ref, mut subuictx) = uictx.derive(&child_id).unwrap(); // unwrap OK b/c NodeId valid & in view

            if within_widget(point, &widget_ref.rect()) {
                if let Some(emittable) = widget_ref.as_emit_event() {
                    emittable.emit(event, &mut subuictx)?;
                    let pane_events = subuictx.collect_child_events();
                    if pane_events.len() != 0 {
                        for child_event in pane_events {
                            child_events.push((widget_ref.id().unwrap().clone(), child_event));
                        }
                        break;
                    } else {
                        return Ok(());
                    }
                } else {
                    debug!("nothing to emit on; widget is not an EmitEvent");
                }
            }
        }

        // Emitted click events clear the previous focus
        if event.what == context::EventType::Click {
            if let Some(current_focused_id) = focus_cycle.focused_widget_id() {
                println!("Clearing due to Click: {:?}", current_focused_id);
                Layering::emit_focus_change(context::EventType::LoseFocus, uictx, current_focused_id)?;
                focus_cycle.clear_focus()
            }
        }

        for (child_id, child_event) in child_events {
            // Gain a new focus
            if child_event.what == context::EventType::ChildRequestsFocus {
                println!("Gaining due to Click: {:?}", child_id);
                Layering::emit_focus_change(context::EventType::GainFocus, uictx, &child_id)?;
                focus_cycle.set_focused(&child_id);
                break;
            }
        }
        Ok(())
    }

    fn handle_unhandled_keyboard_events(
        event: &context::Event,
        uictx: &mut context::UIContext,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(context::KeyCodeOrChar::KeyCode(key)) = event.key {
            match key {
                KeyCode::Escape => {
                    // Handle Escape, only if screen was not changed above
                    let screen = uictx.current_screen();
                    if screen == Screen::Menu && uictx.game_in_progress {
                        uictx.push_screen(Screen::Run);
                    } else {
                        uictx.pop_screen()?;
                    }
                }
                _ => {}
            }
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
            font:            (),                  //dummy font because we can't create a real Font without ggez
            scale:           Scale::uniform(1.0), // Does not matter
            char_dimensions: Vector2::<f32>::new(5.0, 5.0), // any positive values will do
        }
    }

    #[test]
    fn test_add_widget_to_layer_basic() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(font_info, history_len);

        let id = layer_info.add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer);

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

        let id = layer_info.add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer);
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
            .add_widget(Box::new(chatbox), InsertLocation::AtNextLayer)
            .unwrap();

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
            .add_widget(Box::new(chatbox), InsertLocation::AtCurrentLayer)
            .unwrap();

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
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();

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
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
        let chatbox_id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::ToNestedContainer(&pane_id))
            .unwrap();

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
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
        let chatbox_id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::ToNestedContainer(&pane_id))
            .unwrap();

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
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
        let chatbox_id = layer_info
            .add_widget(Box::new(chatbox), InsertLocation::ToNestedContainer(&pane_id))
            .unwrap();

        assert_eq!(layer_info.widget_exists(&pane_id), true);
        assert_eq!(layer_info.widget_exists(&chatbox_id), true);
    }

    #[test]
    fn test_widget_exists_widget_found() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
        assert_eq!(layer_info.widget_exists(&pane_id), true);
    }

    #[test]
    fn test_widget_exists_widget_not_found_list_non_empty() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
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
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);
    }

    #[test]
    fn test_remove_widget_twice_fails() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), false);
    }

    #[test]
    fn test_remove_widget_adds_id_to_hashset() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);

        assert!(layer_info.removed_node_ids.contains(&pane_id));
    }

    #[test]
    fn test_reinserting_widget_fails() {
        let mut layer_info = Layering::new();

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut pane2 = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();
        let _removal = layer_info.remove_widget(pane_id.clone());

        // Try to re-insert with a previously allocated node_id
        pane2.set_id(pane_id);
        assert!(layer_info
            .add_widget(Box::new(pane2), InsertLocation::AtCurrentLayer)
            .is_err());
    }

    #[test]
    fn test_remove_container_widget_adds_children_to_hashset() {
        let mut layer_info = Layering::new();
        let font_info = create_dummy_font();
        let history_len = 5;

        let pane = Pane::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let pane_id = layer_info
            .add_widget(Box::new(pane), InsertLocation::AtCurrentLayer)
            .unwrap();

        let child_node_ids: Vec<NodeId> = (0..5)
            .map(|_| {
                let chatbox = Chatbox::new(font_info, history_len);
                layer_info
                    .add_widget(Box::new(chatbox), InsertLocation::ToNestedContainer(&pane_id))
                    .unwrap()
            })
            .collect();

        let removal = layer_info.remove_widget(pane_id.clone());
        assert_eq!(removal.is_ok(), true);

        let mut all_ids = HashSet::new();
        all_ids.insert(pane_id);
        for node_id in child_node_ids {
            all_ids.insert(node_id);
        }

        assert_eq!(all_ids.difference(&layer_info.removed_node_ids).count(), 0);
    }
}
