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

use ggez::{Context};

use crate::constants::{
    widget_ids::*,
};
use crate::config::Config;
use crate::Screen;
use crate::ui::{
    Widget,
    Button,
    Checkbox, ToggleState,
    Chatbox,
    Layer,
    Pane,
    TextField,
    UIAction,
    helpe,
};

pub struct UILayout {
    pub layers: HashMap<Screen, Vec<Layer>>,
}

impl UILayout {
    pub fn new(ctx: &mut Context, config: &Config, font: Font) -> Self {
        let mut ui_layers = HashMap::new();

        let chat_pane_rect = Rect::new(30.0, 40.0, 300.0, 150.0);
        let mut chatpane = Box::new(Pane::new(INGAME_PANE1, chat_pane_rect));

        const CHATBOX_HISTORY: usize = 5;
        let chatbox_rect = Rect::new(0.0, 0.0, chat_pane_rect.w, chat_pane_rect.h);
        let mut chatbox = Chatbox::new(INGAME_PANE1_CHATBOX,
            font.clone(),
            CHATBOX_HISTORY
        );
        chatbox.set_size(chatbox_rect);
        let chatbox = Box::new(chatbox);

        const CHAT_TEXTFIELD_HEIGHT: f32 = (20.0 + 5.0);
        let textfield_rect = Rect::new(chatbox_rect.x, chatbox_rect.bottom(), chatbox_rect.w, CHAT_TEXTFIELD_HEIGHT);
        let char_dimensions = helpe::get_char_dimensions(ctx, font);
        let textfield = Box::new(TextField::new(INGAME_PANE1_CHATBOXTEXTFIELD,
            font.clone(),
            textfield_rect,
            char_dimensions.x,
        ));

        chatpane.add(chatbox);
        chatpane.add(textfield);

        let checkbox = Box::new(Checkbox::new(ctx, MAINMENU_TESTCHECKBOX,
            UIAction::Toggle( if config.get().video.fullscreen { ToggleState::Enabled } else { ToggleState::Disabled } ),
            font.clone(),
            "Toggle FullScreen".to_owned(),
            Rect::new(10.0, 210.0, 20.0, 20.0),
        ));


        let mut layer_mainmenu = Layer::new(MAINMENU_LAYER1);
        let mut layer_ingame = Layer::new(INGAME_LAYER1);

        // Create a new pane, and add two test buttons to it. Actions do not really matter for now, WIP
        let mut pane = Box::new(Pane::new(MAINMENU_PANE1, Rect::new_i32(20, 20, 300, 250)));
        let mut pane_button = Box::new(Button::new(ctx, MAINMENU_PANE1_BUTTONYES,
            UIAction::ScreenTransition(Screen::ServerList),
            MAINMENU_PANE1_BUTTONYESLABEL,
            font.clone(),
            "ServerList".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 10.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, MAINMENU_PANE1_BUTTONNO,
            UIAction::ScreenTransition(Screen::InRoom),
            MAINMENU_PANE1_BUTTONNOLABEL,
            font.clone(),
            "InRoom".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 70.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, MAINMENU_TESTBUTTON,
            UIAction::ScreenTransition(Screen::Run),
            MAINMENU_TESTBUTTONLABEL,
            font.clone(),
            "StartGame".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 130.0, 180.0, 50.0));
        pane.add(pane_button);

        pane.add(checkbox);

        layer_mainmenu.add(pane);
        layer_ingame.add(chatpane);

        ui_layers.insert(Screen::Menu, vec![layer_mainmenu]);
        ui_layers.insert(Screen::Run, vec![layer_ingame]);

        UILayout {
            layers: ui_layers,
        }
    }
}
