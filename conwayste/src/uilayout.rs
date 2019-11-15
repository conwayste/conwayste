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
    UIResult,
    WidgetID,
};

pub struct UILayout {
    pub layers: HashMap<Screen, Vec<Layer>>,
}

/// `UILayout` is responsible for the definition and storage of UI elements.
impl UILayout {
    pub fn new(ctx: &mut Context, config: &Config, font: Font) -> UIResult<Self> {
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
        chatbox.set_size(chatbox_rect)?;
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

        chatpane.add(chatbox)?;
        chatpane.add(textfield)?;

        let mut layer_mainmenu = Layer::new(MAINMENU_LAYER1);
        let mut layer_ingame = Layer::new(INGAME_LAYER1);

        // Create the main menu pane and add buttons to it
        // TODO: this should be a grid layout so that the x and y positions of the buttons are
        // calculated automatically.
        let (pane_x, pane_y) = (20.0, 20.0);
        let large_value = 9999.0; // we dun' want no errrs
        let pad = 10.0;
        let (button_width, button_height) = (180.0, 50.0);
        let mut pane_mainmenu = Box::new(Pane::new(MAINMENU_PANE1, Rect::new(pane_x, pane_y, large_value, large_value)));

        struct ButtonInfo(
            WidgetID,
            UIAction,
            WidgetID,
            &'static str,
        );
        let button_infos: &[ButtonInfo] = &[
                ButtonInfo(
                    MAINMENU_PANE1_BUTTONSERVLST,
                    UIAction::ScreenTransition(Screen::ServerList),
                    MAINMENU_PANE1_BUTTONSERVLSTLABEL,
                    "ServerList",
                ),
                ButtonInfo(
                    MAINMENU_PANE1_BUTTONINROOM,
                    UIAction::ScreenTransition(Screen::InRoom),
                    MAINMENU_PANE1_BUTTONINROOMLABEL,
                    "InRoom",
                ),
                ButtonInfo(
                    MAINMENU_PANE1_BUTTONSTART,
                    UIAction::ScreenTransition(Screen::Run),
                    MAINMENU_PANE1_BUTTONSTARTLABEL,
                    "StartGame",
                ),
                ButtonInfo(
                    MAINMENU_PANE1_BUTTONOPTIONS,
                    UIAction::ScreenTransition(Screen::Options),
                    MAINMENU_PANE1_BUTTONOPTIONSLABEL,
                    "Options",
                ),
        ];

        let (x, mut y) = (pad, pad);
        for info in button_infos {
            let mut pane_button = Box::new(Button::new(ctx, info.0, info.1, info.2, default_font_info, info.3.to_owned()));
            pane_button.set_size(Rect::new(x, y, button_width, button_height))?;
            pane_mainmenu.add(pane_button)?;
            y += button_height + pad;
        }

        /////
        let (checkbox_width, checkbox_height) = (20.0, 20.0);
        let checkbox = Box::new(
            Checkbox::new(
                ctx,
                MAINMENU_FULLSCREEN_CHECKBOX,
                config.get().video.fullscreen,
                default_font_info,
                "Toggle FullScreen".to_owned(),
                Rect::new(x, y, checkbox_width, checkbox_height),
            )
        );
        pane_mainmenu.add(checkbox)?;
        y += checkbox_height + pad;

        pane_mainmenu.set_size(Rect::new(pane_x, pane_y, pad + button_width + pad, y))?;

        layer_mainmenu.add(pane_mainmenu);
        layer_ingame.add(chatpane);

        ui_layers.insert(Screen::Menu, vec![layer_mainmenu]);
        ui_layers.insert(Screen::Run, vec![layer_ingame]);

        Ok(UILayout {
            layers: ui_layers,
        })
    }
}
