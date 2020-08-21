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

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::mem;

use downcast_rs::Downcast;
use enum_iterator::IntoEnumIterator;
use ggez;
use ggez::event::MouseButton;
use ggez::graphics::Rect;
use ggez::input::keyboard::KeyCode;
use ggez::nalgebra::Point2;
use id_tree::NodeId;

use super::treeview::TreeView;
use super::BoxedWidget;
use crate::config;
use crate::Screen;

/// Stores references to many things a handler is likely to need:
///
/// * `ggez_context` - useful for game engine interactions.
/// * `config` - Conwayste configuration settings.
/// * `widget_view` - a `TreeView` on the handler's widget and all widgets beneath it in the widget tree.
/// * `screen_stack` - the layers of `Screen`s in the UI. Handlers are able to push or pop this stack.
pub struct UIContext<'a> {
    pub ggez_context:     &'a mut ggez::Context,
    pub config:           &'a mut config::Config,
    pub widget_view:      TreeView<'a, BoxedWidget>,
    pub screen_stack:     &'a mut Vec<Screen>,
    pub game_in_progress: bool,
    child_events:         Vec<Event>,
}

impl<'a> UIContext<'a> {
    pub fn new(
        ggez_context: &'a mut ggez::Context,
        config: &'a mut config::Config,
        view: TreeView<'a, BoxedWidget>,
        screen_stack: &'a mut Vec<Screen>,
        game_in_progress: bool,
    ) -> Self {
        UIContext {
            ggez_context,
            config,
            widget_view: view,
            child_events: vec![],
            screen_stack,
            game_in_progress,
        }
    }

    /// Create a new UIContext derived from this one, also returning a mutable reference to a
    /// `Box<dyn Widget>` for widget with the specified `NodeId`. This exists because the
    /// `UIContext` is mutably borrowing a subset of the Widgets in this `Layering` (using a
    /// `TreeView`) and we need a smaller subset to be borrowed. That way, the specified `Widget`
    /// is not double mutably borrow.
    ///
    /// # Errors
    ///
    /// This returns an error in the same cases that `TreeView::sub_tree` returns an error:
    ///
    /// * NodeId is invalid for the underlying Tree.
    /// * NodeId refers to a Node that is outside of this TreeView.
    pub fn derive(&mut self, node_id: &NodeId) -> Result<(&mut BoxedWidget, UIContext), Box<dyn Error>> {
        let (node_ref, subtree) = self.widget_view.sub_tree(node_id)?;
        let widget_ref = node_ref.data_mut();
        Ok((
            widget_ref,
            UIContext {
                ggez_context:     self.ggez_context,
                config:           self.config,
                widget_view:      subtree,
                screen_stack:     self.screen_stack,
                child_events:     vec![],
                game_in_progress: self.game_in_progress,
            },
        ))
    }

    /// Return a Result containing a reference to a `Box<dyn Widget>` for the specified `NodeId` if
    /// it exists and is in view in the tree, or else a `NodeIdError`.
    #[allow(unused)]
    pub fn get(&self, node_id: &NodeId) -> Result<&BoxedWidget, Box<dyn Error>> {
        Ok(self.widget_view.get(node_id)?.data())
    }

    /// Return a Result containing a mutable reference to a `Box<dyn Widget>` for the specified
    /// `NodeId` if it exists and is in view in the tree, or else a `NodeIdError`.
    #[allow(unused)]
    pub fn get_mut(&mut self, node_id: &NodeId) -> Result<&mut BoxedWidget, Box<dyn Error>> {
        Ok(self.widget_view.get_mut(node_id)?.data_mut())
    }

    /// Adds an event to be later collected by the parent of this widget (or one of its parents,
    /// etc.). It must be retrieved by collect_child_events() before this UIContext is dropped.
    pub fn child_event(&mut self, event: Event) {
        self.child_events.push(event);
    }

    /// Retrieve all events from this widget's children. Typically called after `emit` onto a child
    /// widget.
    pub fn collect_child_events(&mut self) -> Vec<Event> {
        let mut events = vec![];
        mem::swap(&mut self.child_events, &mut events);
        events
    }

    /// Gets the current screen.
    ///
    /// # Panics
    ///
    /// This will panic if the screen stack is empty, but that shouldn't ever happen.
    #[allow(unused)]
    pub fn current_screen(&self) -> Screen {
        *self.screen_stack.last().unwrap()
    }

    /// Pops off the current screen on the stack, returning to the screen below it. If successful,
    /// the popped screen is returned.
    ///
    /// # Errors
    ///
    /// This will return an error if the screen stack would become empty as a result.
    #[allow(unused)]
    pub fn pop_screen(&mut self) -> Result<Screen, Box<dyn Error>> {
        if self.screen_stack.len() <= 1 {
            return Err(format!("cannot pop_screen; screen_stack is only {:?}", self.screen_stack).into());
        }
        Ok(self.screen_stack.pop().unwrap())
    }

    /// Pushes a screen onto the screen stack.
    pub fn push_screen(&mut self, screen: Screen) {
        self.screen_stack.push(screen)
    }

    /// Replaces the current screen with a new screen. The screen stack's size does not change. The
    /// previous screen is returned.
    ///
    /// # Panics
    ///
    /// This will panic if the screen stack is empty, but that shouldn't ever happen.
    #[allow(unused)]
    pub fn replace_screen(&mut self, screen: Screen) -> Screen {
        let old_screen = *self.screen_stack.last().unwrap();
        let last_index = self.screen_stack.len() - 1;
        self.screen_stack[last_index] = screen;
        old_screen
    }
}

impl<'a> Drop for UIContext<'a> {
    fn drop(&mut self) {
        if self.child_events.len() > 0 {
            warn!(
                "UIContext dropped but collect_child_events() not called. {} events to collect.",
                self.child_events.len(),
            );
        }
    }
}

/// The type of an event.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, IntoEnumIterator)]
pub enum EventType {
    None,
    Click,
    DoubleClick,
    KeyPress,
    MouseMove,
    Drag,
    MousePressAndHeld,
    Translate,
    Resize,
    ParentTranslate,
    ParentResize,
    GainFocus,
    LoseFocus,
    ChildReleasedFocus,
    ChildRequestsFocus,
    // ChildReleasedFocus goes toward the root of the tree! Emitted by Pane onto its parent via
    // child_event(). Note that a LoseFocus event will not be received after this is sent.
    TextEntered,
    Update,
}

/// Describes a MouseMove event in relation to a Rect.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MoveCross {
    Enter,
    Exit,
    None,
}

/// Represents an event to be used by one or more handlers. The `what` field indicates the type of
/// the event. Each type uses only a subset of the fields in this struct, and creators of `Event`s
/// must ensure that all the fields a handler is expecting are present to avoid panics from
/// unwrapping fields; for this reason, it is recommended to use the type-specific constructors
/// (for example, `Event::new_key_press`).
#[derive(Debug, Clone)]
pub struct Event {
    pub what:          EventType,
    pub point:         Option<Point2<f32>>, // Must not be None if this is a mouse event type
    pub prev_point:    Option<Point2<f32>>, // MouseMove / Drag
    pub button:        Option<MouseButton>, // Click
    pub key:           Option<KeyCodeOrChar>,
    pub shift_pressed: bool,
    pub text:          Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KeyCodeOrChar {
    KeyCode(KeyCode),
    Char(char),
}

/// A slice containing all EventTypes related to the keyboard. Must have a key set.
pub const KEY_EVENTS: &[EventType] = &[EventType::KeyPress];

/// A slice containing all EventTypes related to the mouse.
pub const MOUSE_EVENTS: &[EventType] = &[EventType::Click, EventType::MouseMove, EventType::Drag];

/// A slice containing all EventTypes related to keyboard focus changes.
pub const FOCUS_EVENTS: &[EventType] = &[
    EventType::GainFocus,
    EventType::LoseFocus,
    EventType::ChildReleasedFocus,
];

impl EventType {
    /// Returns true if and only if this is a keyboard event type.
    pub fn is_key_event(self) -> bool {
        KEY_EVENTS.contains(&self)
    }

    /// Returns true if and only if this is a mouse event type. This implies that point is valid.
    pub fn is_mouse_event(self) -> bool {
        MOUSE_EVENTS.contains(&self)
    }

    /// Returns true if and only if this is a keyboard focus event type.
    pub fn is_focus_event(self) -> bool {
        FOCUS_EVENTS.contains(&self)
    }
}

impl Default for Event {
    fn default() -> Self {
        Event {
            what:          EventType::None,
            point:         None,
            prev_point:    None,
            button:        None,
            key:           None,
            shift_pressed: false,
            text:          None,
        }
    }
}

impl Event {
    pub fn new_char_press(mouse_point: Point2<f32>, character: char, is_shift: bool) -> Self {
        Event {
            what: EventType::KeyPress,
            point: Some(mouse_point),
            key: Some(KeyCodeOrChar::Char(character)),
            shift_pressed: is_shift,
            ..Default::default()
        }
    }

    pub fn new_key_press(mouse_point: Point2<f32>, key_code: KeyCode, is_shift: bool) -> Self {
        Event {
            what: EventType::KeyPress,
            point: Some(mouse_point),
            key: Some(KeyCodeOrChar::KeyCode(key_code)),
            shift_pressed: is_shift,
            ..Default::default()
        }
    }

    pub fn new_click(mouse_point: Point2<f32>, mouse_button: MouseButton, is_shift: bool) -> Self {
        Event {
            what: EventType::Click,
            point: Some(mouse_point),
            button: Some(mouse_button),
            shift_pressed: is_shift,
            ..Default::default()
        }
    }

    pub fn new_mouse_move(
        prev_point: Point2<f32>,
        point: Point2<f32>,
        mouse_button: MouseButton,
        is_shift: bool,
    ) -> Self {
        Event {
            what: EventType::MouseMove,
            point: Some(point),
            prev_point: Some(prev_point),
            button: Some(mouse_button),
            shift_pressed: is_shift,
            ..Default::default()
        }
    }

    /// For MouseMove events, indicate whether the mouse entered/exited the given box, or neither.
    /// Use this to implement on-hover displays.
    pub fn move_did_cross(&self, rect: Rect) -> MoveCross {
        if self.what != EventType::MouseMove {
            // Not an error
            return MoveCross::None;
        }

        let previously_inside = rect.contains(self.prev_point.unwrap());
        let currently_inside = rect.contains(self.point.unwrap());
        if previously_inside == currently_inside {
            MoveCross::None
        } else if !previously_inside && currently_inside {
            MoveCross::Enter
        } else {
            MoveCross::Exit
        }
    }

    pub fn new_child_released_focus() -> Self {
        Event {
            what: EventType::ChildReleasedFocus,
            ..Default::default()
        }
    }

    pub fn new_text_entered(text: String) -> Self {
        Event {
            what: EventType::TextEntered,
            text: Some(text),
            ..Default::default()
        }
    }

    /// # Panics
    ///
    /// Will panic if event type is not a GainFocus or LoseFocus
    pub fn new_gain_or_lose_focus(what: EventType) -> Self {
        if what != EventType::GainFocus && what != EventType::LoseFocus {
            panic!("Unexpected event type passed to new_gain_or_lose_focus: {:?}", what);
        }
        Event {
            what,
            ..Default::default()
        }
    }

    pub fn new_child_request_focus() -> Self {
        Event {
            what: EventType::ChildRequestsFocus,
            ..Default::default()
        }
    }

    pub fn new_update() -> Self {
        Event {
            what: EventType::Update,
            ..Default::default()
        }
    }

    pub fn new_drag(mouse_point: Point2<f32>, mouse_button: MouseButton, is_shift: bool) -> Self {
        Event {
            what: EventType::Drag,
            point: Some(mouse_point),
            button: Some(mouse_button),
            shift_pressed: is_shift,
            ..Default::default()
        }
    }

    /// Returns true if and only if this is a keyboard event.
    pub fn is_key_event(&self) -> bool {
        self.what.is_key_event()
    }

    /// Returns true if and only if this is a mouse event. This implies that point is valid.
    pub fn is_mouse_event(&self) -> bool {
        self.what.is_mouse_event()
    }

    /// Returns true if and only if this is a keyboard focus event.
    #[allow(unused)]
    pub fn is_focus_event(self) -> bool {
        self.what.is_focus_event()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Handled {
    Handled,    // no other handlers should be called
    NotHandled, // continue calling handlers
}

pub type Handler = Box<dyn FnMut(&mut dyn EmitEvent, &mut UIContext, &Event) -> Result<Handled, Box<dyn Error>> + Send>;

pub type HandlerMap = HashMap<EventType, Vec<Handler>>;

pub struct HandlerData {
    pub handlers:         Option<HandlerMap>,
    pub forwarded_events: Vec<Event>,
}

impl fmt::Debug for HandlerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let handler_str = if let Some(ref handlers) = self.handlers {
            format!("Some(HandlerMap<{} handlers>)", handlers.len())
        } else {
            "None".to_owned()
        };
        write!(
            f,
            "HandlerData {{ handlers: {}, forwarded_events: {:?} }}",
            handler_str, self.forwarded_events
        )
    }
}

impl HandlerData {
    pub fn new() -> Self {
        HandlerData {
            handlers:         Some(HandlerMap::new()),
            forwarded_events: vec![],
        }
    }
}

/// Trait for widgets that can handle various events. Use `.on` to register a handler and `.emit`
/// to emit an event which will cause all handlers for the event's type to be called.
///
/// Generally, this should be implemented on widgets using impl_emit_event!(...), rather than
/// handwriting implementations for the two required methods.
///
/// Don't return an error from a handler unless things are really screwed up, ok? Also prefer to
/// return NotHandled so that other handlers on this widget can be attached later.
///
/// # Errors
///
/// * It is an error to call `.emit` or `.on` from within a handler.
pub trait EmitEvent: Downcast {
    /// Setup a handler for an event type
    ///
    /// ```
    /// let handler = |obj: &mut dyn EmitEvent, uictx: &mut context::UIContext, evt: &context::Event| {
    ///     use context::Handled::*;
    ///     let mut widget = obj.downcast_mut::<MyWidget>().unwrap();
    ///
    ///     //... do stuff
    ///
    ///     Ok(Handled) // can also return NotHandled to allow other handlers for this event type to run
    /// };
    /// my_widget.on(context::EventType::Click, Box::new(handler));
    /// ```
    ///
    /// # Errors
    ///
    /// * It is an error to call this from within a handler.
    fn on(&mut self, what: EventType, f: Handler) -> Result<(), Box<dyn Error>>;

    /// Emit an event -- call all handlers for this event's type (as long as they return NotHandled)
    ///
    /// # Errors
    ///
    /// * It is an error for a widget's handler to call .emit on itself, unless it supports event
    ///   forwarding.
    /// * The first error to be returned by a handler will be returned here, and no other handlers
    ///   will run.
    fn emit(&mut self, event: &Event, uictx: &mut UIContext) -> Result<Handled, Box<dyn Error>>;
}

impl_downcast!(EmitEvent);

/// Implement EmitEvent for a widget (though strictly speaking non-widgets can implement it).
///
/// # Example
///
/// ```
/// struct MyWidget {
///     handler_data: context::HandlerData,
///     ...
/// }
///
/// impl MyWidget {
///     fn new() -> Self {
///         MyWidget {
///             handlers: context::HandlerData::new(),
///             ...
///         }
///     }
/// }
/// // top level of the module
/// impl_emit_event!(MyWidget, self.handlers);
/// ```
#[macro_export]
macro_rules! impl_emit_event {
    ($widget_name:ty, self.$handler_data_field:ident) => {
        use crate::ui::context::Handled as H;
        use H::*;
        impl crate::ui::context::EmitEvent for $widget_name {
            /// Setup a handler for an event type
            fn on(
                &mut self,
                what: crate::ui::context::EventType,
                hdlr: crate::ui::context::Handler,
            ) -> Result<(), Box<dyn std::error::Error>> {
                let handlers =
                    self.$handler_data_field
                        .handlers
                        .as_mut()
                        .ok_or_else(|| -> Box<dyn std::error::Error> {
                            format!(
                                ".on({:?}, ...) was called while .emit call was in progress for {} widget",
                                what,
                                stringify!($widget_name)
                            )
                            .into()
                        })?;

                let handler_vec: &mut Vec<crate::ui::context::Handler>;
                if let Some(vref) = handlers.get_mut(&what) {
                    handler_vec = vref;
                } else {
                    handlers.insert(what, vec![]);
                    handler_vec = handlers.get_mut(&what).unwrap();
                }
                handler_vec.push(hdlr);
                Ok(())
            }

            /// Emit an event -- call all handlers for this event's type (as long as they return NotHandled)
            fn emit(
                &mut self,
                event: &crate::ui::context::Event,
                uictx: &mut crate::ui::context::UIContext,
            ) -> Result<H, Box<dyn std::error::Error>> {
                let mut event_handled = NotHandled;

                if self.$handler_data_field.handlers.is_none() {
                    // save event into forwarded_events for later forwarding
                    self.$handler_data_field.forwarded_events.push(event.clone());
                    return Ok(NotHandled);
                }

                // take the handlers, so we are not mutably borrowing them more than once during
                // the call to each handler, below.
                let mut handlers = self.$handler_data_field.handlers.take().unwrap(); // unwrap OK b/c .is_none() checked above

                // handle regular (non-forwarded) events
                if let Some(handler_vec) = handlers.get_mut(&event.what) {
                    // call each handler for this event type, until a Handled is returned
                    for hdlr in handler_vec {
                        let handled = hdlr(self, uictx, event)?;
                        if handled == Handled {
                            event_handled = Handled;
                            break;
                        }
                    }
                }

                // handle forwarded events
                loop {
                    let mut events = vec![];
                    std::mem::swap(&mut events, &mut self.$handler_data_field.forwarded_events);
                    if events.len() == 0 {
                        break;
                    }

                    for event in events {
                        if let Some(handler_vec) = handlers.get_mut(&event.what) {
                            // call each handler for this event type, until a Handled is returned
                            for hdlr in handler_vec {
                                let handled = hdlr(self, uictx, &event)?;
                                if handled == Handled {
                                    event_handled = Handled;
                                    break;
                                }
                            }
                        }
                    }
                }

                self.$handler_data_field.handlers = Some(handlers); // put it back
                Ok(event_handled)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_enum_iter() {
        let all: Vec<EventType> = EventType::into_enum_iter().collect();
        assert_eq!(all.len(), EventType::VARIANT_COUNT);
        assert!(all.contains(&EventType::Click));
    }
}
