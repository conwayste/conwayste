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

use std::collections::HashMap;
use std::error::Error;

use ggez::graphics::{Rect, Font};
use ggez::nalgebra::Vector2;
use ggez::Context;

use id_tree::NodeId;

use crate::constants::{self};
use crate::config::Config;
use crate::Screen;
use crate::ui::{
    Widget,
    Button,
    Checkbox,
    Chatbox,
    InsertLocation,
    Layering,
    Pane,
    TextField,
    UIAction,
    UIResult,
    common,
    context,
};

use context::{
    EmitEvent, // so we can call .on(...) on widgets that implement this
    EventType,
};

pub struct UILayout {
    pub layers: HashMap<Screen, Layering>,

    // The fields below correspond to static ui elements that the client may need to interact with
    // regardless of what is displayed on screen. For example, new chat messages should always be
    // forwarded to the UI widget.
    pub chatbox_id: NodeId,
    pub chatbox_tf_id: NodeId,
}

/// `UILayout` is responsible for the definition and storage of UI elements.
impl UILayout {
    pub fn new(ctx: &mut Context, config: &Config, font: Font) -> UIResult<Self> {
        let mut ui_layers = HashMap::new();

        let chat_pane_rect = *constants::DEFAULT_CHATBOX_RECT;
        let mut chatpane = Box::new(Pane::new(chat_pane_rect));
        chatpane.bg_color = Some(*constants::colors::CHAT_PANE_FILL_COLOR);


        let chatbox_rect = Rect::new(
            0.0,
            0.0,
            chat_pane_rect.w,
            chat_pane_rect.h - constants::CHAT_TEXTFIELD_HEIGHT
        );
        let chatbox_font_info = common::FontInfo::new(
            ctx,
            font,
            Some(*constants::DEFAULT_CHATBOX_FONT_SCALE),
        );
        let mut chatbox = Chatbox::new(
            chatbox_font_info,
            constants::CHATBOX_HISTORY
        );
        chatbox.set_rect(chatbox_rect)?;

        let chatbox = Box::new(chatbox);

        let textfield_rect = Rect::new(
            chatbox_rect.x,
            chatbox_rect.bottom(),
            chatbox_rect.w,
            constants::CHAT_TEXTFIELD_HEIGHT
        );
        let default_font_info = common::FontInfo::new(ctx, font, None);
        let mut textfield = Box::new(
            TextField::new(
                default_font_info,
                textfield_rect,
            )
        );
        textfield.bg_color = Some(*constants::colors::CHAT_PANE_FILL_COLOR);

        let mut layer_mainmenu = Layering::new();
        let mut layer_ingame = Layering::new();

        // Create a new pane, and add two test buttons to it.
        let pane = Box::new(Pane::new(Rect::new_i32(20, 20, 410, 250)));
        let mut serverlist_button = Box::new(
            Button::new(
                ctx,
                UIAction::ScreenTransition(Screen::ServerList),
                default_font_info,
                "ServerList".to_owned()
            )
        );
        serverlist_button.set_rect(Rect::new(10.0, 10.0, 180.0, 50.0))?;
        register_test_click_handler(&mut serverlist_button, "Server List!".to_owned()).unwrap(); // XXX

        let mut inroom_button = Box::new(
            Button::new(
                ctx,
                UIAction::ScreenTransition(Screen::InRoom),
                default_font_info,
                "InRoom".to_owned()
            )
        );
        inroom_button.set_rect(Rect::new(10.0, 70.0, 180.0, 50.0))?;
        register_test_click_handler(&mut inroom_button, "In Room!".to_owned()).unwrap(); // XXX

        let mut startgame_button = Box::new(
            Button::new(
                ctx,
                UIAction::ScreenTransition(Screen::Run),
                default_font_info,
                "StartGame".to_owned()
            )
        );
        startgame_button.set_rect(Rect::new(10.0, 130.0, 180.0, 50.0))?;
        register_test_click_handler(&mut startgame_button, "Start Game!".to_owned()).unwrap(); // XXX

        let mut fullscreen_checkbox = Box::new(
            Checkbox::new(
                ctx,
                config.get().video.fullscreen,
                default_font_info,
                "Toggle FullScreen".to_owned(),
                Rect::new(10.0, 210.0, 20.0, 20.0),
            )
        );
        // unwrap OK here because we are not calling .on from within a handler
        fullscreen_checkbox.on(EventType::Click, Box::new(fullscreen_toggle_handler)).unwrap();

        // TODO: delete this once handlers are tested
        let mut handler_test_button = Box::new(
            Button::new(
                ctx,
                UIAction::None,
                default_font_info,
                "Handler Test".to_owned()
            )
        );
        // add the handler!
        // TODO: delete this
        // unwrap OK here because we are not calling .on from within a handler
        handler_test_button.on(EventType::Click, Box::new(test_handler)).unwrap();

        match handler_test_button.set_rect(Rect::new(200.0, 130.0, 180.0, 50.0)) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for button during initialization! {:?}", e);
            }
        }

        let menupane_id = layer_mainmenu.add_widget(pane, InsertLocation::AtCurrentLayer)?;
        layer_mainmenu.add_widget(startgame_button, InsertLocation::ToNestedContainer(&menupane_id))?;
        layer_mainmenu.add_widget(inroom_button, InsertLocation::ToNestedContainer(&menupane_id))?;
        layer_mainmenu.add_widget(serverlist_button, InsertLocation::ToNestedContainer(&menupane_id))?;
        layer_mainmenu.add_widget(handler_test_button, InsertLocation::ToNestedContainer(&menupane_id))?;
        layer_mainmenu.add_widget(fullscreen_checkbox, InsertLocation::ToNestedContainer(&menupane_id))?;

        let chatpane_id = layer_ingame.add_widget(chatpane, InsertLocation::AtCurrentLayer)?;
        let chatbox_id = layer_ingame.add_widget(chatbox, InsertLocation::ToNestedContainer(&chatpane_id))?;
        let chatbox_tf_id = layer_ingame.add_widget(textfield, InsertLocation::ToNestedContainer(&chatpane_id))?;

        ui_layers.insert(Screen::Menu, layer_mainmenu);
        ui_layers.insert(Screen::Run, layer_ingame);

        Ok(UILayout {
            layers: ui_layers,
            chatbox_id,
            chatbox_tf_id,
        })
    }
}
fn fullscreen_toggle_handler(obj: &mut dyn EmitEvent, uictx: &mut context::UIContext, _evt: &context::Event) -> Result<context::Handled, Box<dyn Error>> {
    use context::Handled::*;

    // NOTE: the checkbox installed its own handler to toggle the `enabled` field on click
    // We are running after it, since the handler registered first gets called first.

    let checkbox = obj.downcast_ref::<Checkbox>().unwrap();

    uictx.config.modify(|settings| {
        settings.video.fullscreen = checkbox.enabled;
    });
    Ok(Handled)
}

fn test_handler(obj: &mut dyn EmitEvent, uictx: &mut context::UIContext, evt: &context::Event) -> Result<context::Handled, Box<dyn Error>> {
    use context::Handled::*;
    let btn = obj.downcast_mut::<Button>().unwrap();

    info!("YAYYYY BUTTON'S HANDLER CALLED!!!");

    btn.translate(Vector2::new(1.0, 1.0)); // just for fun, move it diagonally by one pixel

    // get the number of ticks, also just for fun
    let num_ticks = ggez::timer::ticks(&uictx.ggez_context);
    info!("number of ggez ticks: {}", num_ticks);

    // ok now let's print out the event
    info!("EVENT: what={:?} @ {}", evt.what, evt.point.unwrap());

    Ok(Handled)
}

fn register_test_click_handler(button: &mut Button, text: String) -> Result<(), Box<dyn Error>> {
    let handler = move |obj: &mut dyn EmitEvent, _uictx: &mut context::UIContext, evt: &context::Event|  -> Result<context::Handled, Box<dyn Error>> {
        use context::Handled::*;
        let btn = obj.downcast_mut::<Button>().unwrap(); // unwrap OK because this is only registered on a button
        info!("test click handler: button {:?}: {}", btn.id(), text);
        info!("test click handler: button {:?}: ^^^ click at {:?}", btn.id(), evt.point);
        Ok(Handled)
    };
    button.on(EventType::Click, Box::new(handler))?;

    Ok(())
}
