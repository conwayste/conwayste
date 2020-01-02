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

use ggez::graphics::{Rect, Font};
use ggez::nalgebra::Vector2;

use ggez::{Context};

use crate::constants::{
    self,
    widget_ids::*,
};
use crate::config::Config;
use crate::Screen;
use crate::ui::{
    Widget,
    Button,
    Checkbox,
    Chatbox,
    Layer,
    Pane,
    TextField,
    UIAction,
    common,
    context,
};

use context::{
    EmitEvent, // so we can call .on(...) on widgets that implement this
    Handler,
    EventType,
};

pub struct UILayout {
    pub layers: HashMap<Screen, Vec<Layer>>,
}

/// `UILayout` is responsible for the definition and storage of UI elements.
impl UILayout {
    pub fn new(ctx: &mut Context, config: &Config, font: Font) -> Self {
        let mut ui_layers = HashMap::new();

        let chat_pane_rect = *constants::DEFAULT_CHATBOX_RECT;
        let mut chatpane = Box::new(Pane::new(INGAME_PANE1, chat_pane_rect));
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
            INGAME_PANE1_CHATBOX,
            chatbox_font_info,
            constants::CHATBOX_HISTORY
        );
        match chatbox.set_rect(chatbox_rect) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for chatbox during initialization! {:?}", e);
            }
        }
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
                INGAME_PANE1_CHATBOXTEXTFIELD,
                default_font_info,
                textfield_rect,
            )
        );
        textfield.bg_color = Some(*constants::colors::CHAT_PANE_FILL_COLOR);

        match chatpane.add(chatbox) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    INGAME_PANE1_CHATBOX,
                    INGAME_PANE1,
                    e
                );
            }
        }
        match chatpane.add(textfield) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    INGAME_PANE1_CHATBOXTEXTFIELD,
                    INGAME_PANE1,
                    e
                );
            }
        }

        let mut layer_mainmenu = Layer::new(MAINMENU_LAYER1);
        let mut layer_ingame = Layer::new(INGAME_LAYER1);

        // Create a new pane, and add two test buttons to it.
        let mut pane = Box::new(Pane::new(MAINMENU_PANE1, Rect::new_i32(20, 20, 410, 250)));
        let mut pane_button = Box::new(
            Button::new(
                ctx,
                MAINMENU_PANE1_BUTTONYES,
                UIAction::ScreenTransition(Screen::ServerList),
                MAINMENU_PANE1_BUTTONYESLABEL,
                default_font_info,
                "ServerList".to_owned()
            )
        );
        match pane_button.set_rect(Rect::new(10.0, 10.0, 180.0, 50.0)) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for button during initialization! {:?}, {:?}",
                    MAINMENU_PANE1_BUTTONYES,
                    e
                );
            }
        }

        match pane.add(pane_button) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    MAINMENU_PANE1_BUTTONYES,
                    MAINMENU_PANE1,
                    e
                );
            }
        }

        let mut pane_button = Box::new(
            Button::new(
                ctx,
                MAINMENU_PANE1_BUTTONNO,
                UIAction::ScreenTransition(Screen::InRoom),
                MAINMENU_PANE1_BUTTONNOLABEL,
                default_font_info,
                "InRoom".to_owned()
            )
        );
        match pane_button.set_rect(Rect::new(10.0, 70.0, 180.0, 50.0)) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for button during initialization! {:?} {:?}",
                MAINMENU_PANE1_BUTTONNO,
                e);
            }
        }
        match pane.add(pane_button) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    MAINMENU_PANE1_BUTTONNO,
                    MAINMENU_PANE1,
                    e
                );
            }
        }

        let mut pane_button = Box::new(
            Button::new(
                ctx,
                MAINMENU_TESTBUTTON,
                UIAction::ScreenTransition(Screen::Run),
                MAINMENU_TESTBUTTONLABEL,
                default_font_info,
                "StartGame".to_owned()
            )
        );
        match pane_button.set_rect(Rect::new(10.0, 130.0, 180.0, 50.0)) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for button during initialization! {:?} {:?}",
                MAINMENU_TESTBUTTON,
                e);
            }
        }
        match pane.add(pane_button) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    MAINMENU_TESTBUTTON,
                    MAINMENU_PANE1,
                    e
                );
            }
        }

        let checkbox = Box::new(
            Checkbox::new(
                ctx,
                MAINMENU_TESTCHECKBOX,
                config.get().video.fullscreen,
                default_font_info,
                "Toggle FullScreen".to_owned(),
                Rect::new(10.0, 210.0, 20.0, 20.0),
            )
        );
        match pane.add(checkbox) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    MAINMENU_TESTCHECKBOX,
                    MAINMENU_PANE1,
                    e
                );
            }
        }
        // TODO: delete this once handlers are tested
        let mut handler_test_button = Box::new(
            Button::new(
                ctx,
                MAINMENU_HDLRTESTBUTTON,
                UIAction::None,
                MAINMENU_HDLRTESTBUTTONLABEL,
                default_font_info,
                "Handler Test".to_owned()
            )
        );
        // add the handler!
        let handler: Handler = Box::new(|obj, uictx, evt| {
            use context::Handled::*;
            let uictx = uictx.unwrap_update();
            let mut btn = obj.downcast_mut::<Button>().unwrap();

            info!("YAYYYY BUTTON'S HANDLER CALLED!!!");

            btn.translate(Vector2::new(1.0, 1.0)); // just for fun, move it diagonally by one pixel

            // get the number of ticks, also just for fun
            let num_ticks = ggez::timer::ticks(&uictx.ggez_context);
            info!("number of ggez ticks: {}", num_ticks);

            // ok now let's print out the event
            info!("EVENT: what={:?} @ ({}, {})", evt.what, evt.x, evt.y);

            Ok(Handled)
        });
        // unwrap OK here because we are not calling .on from within a handler
        handler_test_button.on(EventType::Click, handler).unwrap();

        match handler_test_button.set_rect(Rect::new(200.0, 130.0, 180.0, 50.0)) {
            Ok(()) => { },
            Err(e) => {
                error!("Could not set size for button during initialization! {:?} {:?}",
                MAINMENU_HDLRTESTBUTTON,
                e);
            }
        }
        match pane.add(handler_test_button) {
            Ok(()) => {},
            Err(e) => {
                error!("Could not add widget {:?} to pane {:?}! {:?}",
                    MAINMENU_HDLRTESTBUTTON,
                    MAINMENU_PANE1,
                    e
                );
            }
        }

        layer_mainmenu.add(pane);
        layer_ingame.add(chatpane);

        ui_layers.insert(Screen::Menu, vec![layer_mainmenu]);
        ui_layers.insert(Screen::Run, vec![layer_ingame]);

        UILayout {
            layers: ui_layers,
        }
    }
}
