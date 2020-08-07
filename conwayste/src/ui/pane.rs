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
use std::fmt;

use ggez::graphics::{self, Color, DrawMode, DrawParam, Rect};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use enum_iterator::IntoEnumIterator;
use id_tree::NodeId;

use super::{
    common::within_widget,
    context,
    focus::{CycleType, FocusCycle},
    widget::Widget,
    UIError, UIResult,
};

use ggez::input::keyboard::KeyCode;

use context::{EmitEvent, Event, EventType, Handled, UIContext};

use crate::constants::colors::*;

pub struct Pane {
    id:               Option<NodeId>,
    z_index:          usize,
    pub dimensions:   Rect,
    pub floating:     bool, // can the window be dragged around?
    pub previous_pos: Option<Point2<f32>>,
    pub border:       f32,
    pub bg_color:     Option<Color>,
    pub focus_cycle:  FocusCycle,
    pub handler_data: context::HandlerData, // required for impl_emit_event!

                                            // might need something to track mouse state to see if
                                            // we are still clicked within the boundaries of the
                                            // pane in the dragging case
}

impl fmt::Debug for Pane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pane {{ id: {:?}, z-index: {}, Dimensions: {:?} }}",
            self.id, self.z_index, self.dimensions
        )
    }
}

/// A container of one or more widgets
impl Pane {
    /// Specify the dimensional bounds of the Pane container
    pub fn new(dimensions: Rect) -> Self {
        let mut pane = Pane {
            id: None,
            z_index: std::usize::MAX,
            dimensions,
            floating: true,
            previous_pos: None,
            border: 1.0,
            bg_color: None,
            focus_cycle: FocusCycle::new(CycleType::OpenEnded),
            handler_data: context::HandlerData::new(),
        };

        // for each event type, define a handler of the appropriate type (mouse or keyboard)
        for event_type in EventType::into_enum_iter() {
            if event_type.is_mouse_event() {
                let handler = |_obj: &mut dyn EmitEvent,
                               uictx: &mut context::UIContext,
                               evt: &context::Event|
                 -> Result<Handled, Box<dyn Error>> {
                    for child_id in uictx.widget_view.children_ids() {
                        let (widget_ref, mut subuictx) = uictx.derive(&child_id).unwrap(); // unwrap OK because 1) valid ID, 2) in view

                        let point = &evt.point.unwrap(); // unwrap OK because a Click event always has a point
                        if within_widget(&point, &widget_ref.rect()) {
                            if let Some(emittable_ref) = widget_ref.as_emit_event() {
                                emittable_ref.emit(evt, &mut subuictx)?;
                                let pane_events = subuictx.collect_child_events();
                                if pane_events.len() != 0 {
                                    for event in pane_events {
                                        uictx.child_event(event);
                                    }
                                    warn!(
                                        "expected no mouse child events to be collected from Pane; got {:?}",
                                        pane_events
                                    );
                                }
                                return Ok(Handled::Handled);
                            } else {
                                warn!(
                                    "Widget at point of click ({:?}) does not implement EmitEvent: {:?}",
                                    evt.point,
                                    widget_ref.id(),
                                );
                            }
                        }
                    }
                    Ok(Handled::NotHandled)
                };
                pane.on(event_type, Box::new(handler)).unwrap(); // unwrap OK because we aren't calling from within a handler
            } else if event_type.is_key_event() {
                // unwrap OK because we aren't calling from within a handler
                pane.on(event_type, Box::new(Pane::key_press_handler)).unwrap();
            } else {
                // nothing to do if this is not a key or a mouse event
            }

            pane.on(EventType::Update, Box::new(Pane::broadcast_handler)).unwrap(); // unwrap OK because not called w/in handler
            pane.on(EventType::MouseMove, Box::new(Pane::broadcast_handler))
                .unwrap(); // unwrap OK because not called w/in handler
        }

        // Set handler for focusing first widget in focus cycle when focus is gained
        let gain_focus_handler =
            move |obj: &mut dyn EmitEvent, uictx: &mut UIContext, _evt: &Event| -> Result<Handled, Box<dyn Error>> {
                let pane = obj.downcast_mut::<Pane>().unwrap(); // unwrap OK
                if pane.focus_cycle.focused_widget_id().is_none() {
                    pane.focus_cycle.focus_next();
                }
                if let Some(focused_widget_id) = pane.focus_cycle.focused_widget_id() {
                    let focused_widget_id = focused_widget_id.clone();
                    pane.emit_focus_change(EventType::GainFocus, uictx, &focused_widget_id)?;
                }
                Ok(Handled::NotHandled)
            };
        pane.on(EventType::GainFocus, Box::new(gain_focus_handler)).unwrap(); // unwrap OK

        pane
    }

    fn broadcast_handler(
        _obj: &mut dyn EmitEvent,
        uictx: &mut UIContext,
        event: &Event,
    ) -> Result<Handled, Box<dyn Error>> {
        for child_id in uictx.widget_view.children_ids() {
            // Get a mutable reference to a BoxedWidget, as well as a UIContext with a view on the
            // widgets in the tree under this widget.
            let (widget_ref, mut subuictx) = uictx.derive(&child_id).unwrap(); // unwrap OK b/c NodeId valid & in view

            if let Some(emittable) = widget_ref.as_emit_event() {
                emittable.emit(event, &mut subuictx)?;
                let pane_events = subuictx.collect_child_events();
                if pane_events.len() != 0 {
                    warn!(
                        "[Pane] expected no {:?} child events to be collected from child widget; got {:?}",
                        event.what, pane_events
                    );
                }
            }
        }
        Ok(Handled::NotHandled)
    }

    fn key_press_handler(
        obj: &mut dyn EmitEvent,
        uictx: &mut UIContext,
        event: &Event,
    ) -> Result<Handled, Box<dyn Error>> {
        let key = event
            .key
            .ok_or_else(|| -> Box<dyn Error> { format!("pane event of type {:?} has no key", event.what).into() })?;

        let pane = obj.downcast_mut::<Pane>().unwrap();

        if key == context::KeyCodeOrChar::KeyCode(KeyCode::Tab) {
            // special key press logic to handle focus changes

            let opt_child_id = pane
                .focus_cycle
                .focused_widget_id()
                .map(|child_id_ref| child_id_ref.clone());

            // If a widget is focused, this is Some(<referenced to boxed focused widget>).
            let opt_widget = opt_child_id
                .as_ref()
                .map(|child_id| uictx.widget_view.get(child_id).unwrap().data());
            if opt_child_id.is_some() && opt_widget.unwrap().downcast_ref::<Pane>().is_some() {
                // there is a focused child pane

                let child_id = opt_child_id.unwrap();
                let pane_events = Pane::emit_keyboard_event(event, uictx, &child_id)?;

                pane.handle_events_from_child(uictx, &pane_events[..], event.shift_pressed)?;
            } else {
                // either no focused child widget, or there is but it's not a Pane
                if event.shift_pressed {
                    pane.focus_cycle.focus_previous();
                } else {
                    pane.focus_cycle.focus_next();
                }

                // Only send gain/lose events if the newly focused widget is different from the
                // previously focused widget.
                if pane.focus_cycle.focused_widget_id() != opt_child_id.as_ref() {
                    // send a GainFocus event to the newly focused widget (if any)
                    if let Some(newly_focused_id) = pane.focus_cycle.focused_widget_id() {
                        let newly_focused_id = newly_focused_id.clone();
                        pane.emit_focus_change(EventType::GainFocus, uictx, &newly_focused_id)?;
                    }

                    // send a LoseFocus event to the previously focused widget (if any)
                    if let Some(newly_focused_id) = opt_child_id {
                        pane.emit_focus_change(EventType::LoseFocus, uictx, &newly_focused_id)?;
                    }
                }
            }

            if pane.focus_cycle.focused_widget_id().is_none() {
                // we lost focus; send ChildReleasedFocus event to parent
                let event = Event::new_child_released_focus();
                uictx.child_event(event);
            }
        } else {
            // regular key press logic (no focus changes)
            let focused_id = pane.focus_cycle.focused_widget_id();
            if let Some(id) = focused_id {
                let pane_events = Pane::emit_keyboard_event(event, uictx, id)?;
                pane.handle_events_from_child(uictx, &pane_events[..], event.shift_pressed)?;
            }
        }
        Ok(Handled::Handled)
    }

    fn handle_events_from_child(
        &mut self,
        uictx: &mut UIContext,
        child_events: &[Event],
        shift_pressed: bool,
    ) -> Result<(), Box<dyn Error>> {
        for child_event in child_events {
            // ignore all event types except this one for now
            if child_event.what == context::EventType::ChildReleasedFocus {
                if shift_pressed {
                    self.focus_cycle.focus_previous();
                } else {
                    self.focus_cycle.focus_next();
                }
                // send a GainFocus event to the newly focused widget (if any)
                if let Some(newly_focused_id) = self.focus_cycle.focused_widget_id() {
                    let newly_focused_id = newly_focused_id.clone();
                    self.emit_focus_change(EventType::GainFocus, uictx, &newly_focused_id)?;
                    let more_child_events = uictx.collect_child_events();
                    if more_child_events.len() > 0
                        && more_child_events[0].what == context::EventType::ChildReleasedFocus
                    {
                        error!("[Pane] handle_events_from_child: refusing to recursively handle gain focus / child release focus event loop");
                    }
                }
                break;
            }
        }
        if self.focus_cycle.focused_widget_id().is_none() {
            // we lost focus; send ChildReleasedFocus event to parent
            let event = Event::new_child_released_focus();
            uictx.child_event(event);
        }
        Ok(())
    }

    /// Forward this keyboard event to the specified child widget.
    fn emit_keyboard_event(
        event: &context::Event,
        uictx: &mut UIContext,
        focused_id: &NodeId,
    ) -> Result<Vec<Event>, Box<dyn Error>> {
        let (widget_ref, mut subuictx) = uictx.derive(&focused_id).unwrap(); // unwrap OK b/c NodeId valid & in view
        if let Some(emittable) = widget_ref.as_emit_event() {
            return emittable
                .emit(event, &mut subuictx)
                .map(|_| subuictx.collect_child_events());
        } else {
            // We probably won't ever get here due to the FocusCycle only holding widgets that can
            // receive keyboard events.
            debug!("nothing to emit on; widget is not an EmitEvent");
        }
        Ok(vec![])
    }

    /// Emit a GainFocus or LoseFocus event on the specified child widget.
    fn emit_focus_change(
        &mut self,
        what: EventType,
        uictx: &mut UIContext,
        focused_id: &NodeId,
    ) -> Result<(), Box<dyn Error>> {
        if what != EventType::GainFocus && what != EventType::LoseFocus {
            return Err(format!("Unexpected event type passed to Pane::emit_focus_change: {:?}", what).into());
        }
        let (widget_ref, mut subuictx) = uictx.derive(&focused_id).unwrap(); // unwrap OK b/c NodeId valid & in view
        if let Some(emittable) = widget_ref.as_emit_event() {
            let event = Event::new_gain_or_lose_focus(what);
            emittable.emit(&event, &mut subuictx)?;
            let pane_events = subuictx.collect_child_events();
            self.handle_events_from_child(&mut subuictx, &pane_events[..], false)?;
            let sub_child_events = subuictx.collect_child_events();
            drop(subuictx);

            // Pass on any child events from subuictx to uictx
            for sub_event in sub_child_events {
                uictx.child_event(sub_event);
            }
            return Ok(());
        } else {
            // We probably won't ever get here due to the FocusCycle only holding widgets that can
            // receive keyboard events.
            debug!("nothing to emit on; widget is not an EmitEvent");
        }
        Ok(())
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

    /* PR_GATE: Need to to use the TreeView that has since been added

    /// Adds the vector of children to the parent, and shrink the parent to fit the widgets with
    /// padding on all sides. The widgets will have the padding added to their x and y coordinates.
    /// Therefore it is suggested that the upper left widget in `children` be at `(0, 0)`.
    ///
    /// # Errors
    ///
    /// Any errors from adding are passed down. NOTE: parent will have already been resized!
    //TODO if a `Container` trait is added, this can be a method of that trait instead. This would
    // allow things other than Pane to contain child Widgets.
    pub fn add_and_shrink_to_fit(&mut self, children: Vec<Box<dyn Widget>>, padding: f32) -> UIResult<()> {
        // find bounding box
        let (mut width, mut height) = (0.0, 0.0);
        for child in &children {
            let child_rect = child.rect();
            let w = child_rect.x + child_rect.w;
            let h = child_rect.y + child_rect.h;
            if w > width {
                width = w;
            }
            if h > height {
                height = h;
            }
        }

        // resize parent (use padding)
        self.set_size(width + 2.0 * padding, height + 2.0 * padding)?;

        for mut child in children {
            // add padding to each child
            let mut p = child.position();
            p.x += padding;
            p.y += padding;
            child.set_position(p.x, p.y);

            self.add_widget(child.id())?;
        }
        Ok(())
    }
    */

    /// Add a widget ID to Pane's focus cycle. Must only be called if the widget accepts keyboard
    /// events.
    pub fn add_widget(&mut self, widget_id: NodeId) {
        self.focus_cycle.push(widget_id);
    }

    pub fn remove_widget(&mut self, widget_id: &NodeId) {
        self.focus_cycle.remove(widget_id);
    }
}

impl Widget for Pane {
    fn id(&self) -> Option<&NodeId> {
        self.id.as_ref()
    }

    fn set_id(&mut self, new_id: NodeId) {
        self.id = Some(new_id);
    }

    fn z_index(&self) -> usize {
        self.z_index
    }

    fn set_z_index(&mut self, new_z_index: usize) {
        self.z_index = new_z_index;
    }

    fn rect(&self) -> Rect {
        self.dimensions
    }

    fn set_rect(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!(
                    "Cannot set the size of a Pane {:?} to a width or height of zero",
                    self.id()
                ),
            }));
        }

        self.dimensions = new_dims;
        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.dimensions.point().into()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.dimensions.x = x;
        self.dimensions.y = y;
    }

    fn size(&self) -> (f32, f32) {
        (self.dimensions.w, self.dimensions.h)
    }

    fn set_size(&mut self, w: f32, h: f32) -> UIResult<()> {
        if w == 0.0 || h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions {
                reason: format!("Cannot set the width or height of Pane {:?} to zero", self.id()),
            }));
        }

        self.dimensions.w = w;
        self.dimensions.h = h;

        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>) {
        self.dimensions.translate(dest);
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
                if within_widget(original_pos, &widget.rect()) && self.previous_pos.is_none() {
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
        if let Some(bg_color) = self.bg_color {
            let mesh = graphics::Mesh::new_rectangle(ctx, DrawMode::fill(), self.dimensions, bg_color)?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        if self.border > 0.0 {
            let mesh = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), self.dimensions, *PANE_BORDER_COLOR)?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        Ok(())
    }

    /// convert to EmitEvent
    fn as_emit_event(&mut self) -> Option<&mut dyn context::EmitEvent> {
        Some(self)
    }

    /// Pane can receive keyboard focus because it can contain widgets that can receive focus.
    fn accepts_keyboard_events(&self) -> bool {
        true
    }
}

widget_from_id!(Pane);
impl_emit_event!(Pane, self.handler_data);
