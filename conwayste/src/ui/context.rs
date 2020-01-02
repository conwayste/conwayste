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

use std::error::Error;
use std::collections::HashMap;

use crate::config;
use ggez;

pub enum UIContext<'a> {
    Draw(DrawContext<'a>),
    Update(UpdateContext<'a>),
}

impl<'a> UIContext<'a> {
    pub fn unwrap_draw(&mut self) -> &mut DrawContext<'a> {
        match *self {
            UIContext::Draw(ref mut draw_context) => draw_context,
            _ => panic!("Failed to unwrap DrawContext"),
        }
    }

    pub fn unwrap_update(&mut self) -> &mut UpdateContext<'a> {
        match *self {
            UIContext::Update(ref mut update_context) => update_context,
            _ => panic!("Failed to unwrap UpdateContext"),
        }
    }

    pub fn new_draw(ggez_context: &'a mut ggez::Context, config: &'a config::Config) -> Self {
        UIContext::Draw(DrawContext {
            ggez_context,
            config,
        })
    }

    pub fn new_update(ggez_context: &'a mut ggez::Context, config: &'a mut config::Config) -> Self {
        UIContext::Update(UpdateContext {
            ggez_context,
            config,
        })
    }
}

pub struct DrawContext<'a> {
    pub ggez_context: &'a mut ggez::Context,
    pub config: &'a config::Config,
}

pub struct UpdateContext<'a> {
    pub ggez_context: &'a mut ggez::Context,
    pub config: &'a mut config::Config,
}

// TODO: move this elsewhere; it's in here to keep separate from other code (avoid merge conflicts)
#[allow(unused)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Click,
    KeyPress,
    Move, // mouse move
    Translate,
    Resize,
    ParentTranslate,
    ParentResize,
    // TODO: not sure about Child* because we'd need a widget ID to say which child
    //ChildTranslate,
    //ChildResize,
}

// TODO: move this elsewhere; it's in here to keep separate from other code (avoid merge conflicts)
#[derive(Debug, Clone)]
pub struct Event {
    pub what: EventType,
    pub x: f32,  // usually x position, but width if resize event
    pub y: f32,
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Handled {
    Handled,  // no other handlers should be called
    NotHandled, // continue calling handlers
}

// NOTE: typically widgets will want Handler<Self>
pub type Handler<T> = Box<dyn FnMut(&mut T, &mut UIContext, &Event) -> Result<Handled, Box<dyn Error>> + Send>;

// NOTE: typically widgets will want HandlerMap<Self>
pub type HandlerMap<T> = HashMap<EventType, Vec<Handler<T>>>;

/// Trait for widgets that can handle various events. Use `.on` to register a handler and `.emit`
/// to emit an event which will cause all handlers for the event's type to be called.
///
/// # Errors
///
/// * It is an error to call `.emit` or `.on` from within a handler.
pub trait EmitEvent {
    /// Setup a handler for an event type
    ///
    /// ```
    /// let handler = |w: &mut MyWidget, uictx: &mut context::UIContext, evt: &context::Event| {
    ///     use context::Handled::*;
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
    fn on(&mut self, what: EventType, f: Handler<Self>) -> Result<(), Box<dyn Error>>;

    /// Emit an event -- call all handlers for this event's type (as long as they return NotHandled)
    ///
    /// # Errors
    ///
    /// * It is an error to call this from within a handler.
    /// * The first error to be returned by a handler will be returned here, and no other handlers
    ///   will run.
    fn emit(&mut self, event: &Event, uictx: &mut UIContext) -> Result<(), Box<dyn Error>>;
}

/// Implement EmitEvent for a widget (though strictly speaking non-widgets can implement it).
#[macro_export]
macro_rules! impl_emit_event {
    ($widget_name:ty, self.$handler_field:ident) => {
        impl crate::ui::context::EmitEvent for $widget_name {
            /// Setup a handler for an event type
            fn on(&mut self, what: crate::ui::context::EventType, hdlr: crate::ui::context::Handler<Self>) -> Result<(), Box<dyn std::error::Error>> {
                let handlers = self.$handler_field
                    .as_mut()
                    .ok_or_else(|| -> Box<dyn std::error::Error> {
                        format!("during .on({:?}, ...): a .emit call is in progress for {} widget", what, stringify!($widget_name)).into()
                    })?;

                let handler_vec: &mut Vec<crate::ui::context::Handler<Self>>;
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
            fn emit(&mut self, event: &crate::ui::context::Event, uictx: &mut crate::ui::context::UIContext) -> Result<(), Box<dyn std::error::Error>> {
                use crate::ui::context::Handled::*;
                // HACK: prevent a borrow error when calling handlers
                let mut handlers = self.$handler_field
                    .take()
                    .ok_or_else(|| -> Box<dyn std::error::Error> {
                        format!("during .on({:?}, ...): a .emit call is in progress for {} widget",
                                event.what,
                                stringify!($widget_name)).into()
                    })?;

                if let Some(handler_vec) = handlers.get_mut(&event.what) {
                    // call each handler for this event type, until a Handled is returned
                    for hdlr in handler_vec {
                        let handled = hdlr(self, uictx, event)?;
                        if handled == Handled {
                            break;
                        }
                    }
                }
                self.$handler_field = Some(handlers); // put it back
                Ok(())
            }
        }
    };
}

