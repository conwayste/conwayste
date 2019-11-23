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
use ggez::nalgebra::Point2;

use ggez::{Context};

use crate::constants::{
    self,
    widget_ids::*,
};
use crate::config::Config;
use crate::Screen;
use crate::ui::{
    Button,
    Chatbox,
    Checkbox,
    Label,
    Layer,
    Pane,
    TextField,
    UIAction,
    UIResult,
    Widget,
    WidgetID,
    common,
};

pub struct UILayout {
    pub layers: HashMap<Screen, Vec<Layer>>,
}

/// `UILayout` is responsible for the definition and storage of UI elements.
impl UILayout {
    pub fn new(ctx: &mut Context, config: &Config, font: Font) -> UIResult<Self> {
        let mut ui_layers = HashMap::new();

        let default_font_info = common::FontInfo::new(ctx, font, None);

        ////// In-Game ///////
        let mut layer_ingame = Layer::new(INGAME_LAYER1);
        let pane_chatbox_rect = *constants::DEFAULT_CHATBOX_RECT;
        let mut pane_chatbox = Box::new(Pane::new(INGAME_PANE1, pane_chatbox_rect));
        pane_chatbox.bg_color = Some(*constants::colors::CHAT_PANE_FILL_COLOR);

        let chatbox_rect = Rect::new(
            0.0,
            0.0,
            pane_chatbox_rect.w,
            pane_chatbox_rect.h - constants::TEXTFIELD_HEIGHT
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
        chatbox.set_rect(chatbox_rect)?;
        let chatbox = Box::new(chatbox);

        let textfield_rect = Rect::new(
            chatbox_rect.x,
            chatbox_rect.bottom(),
            chatbox_rect.w,
            constants::TEXTFIELD_HEIGHT
        );
        let mut textfield = Box::new(
            TextField::new(
                INGAME_PANE1_CHATBOXTEXTFIELD,
                default_font_info,
                textfield_rect,
            )
        );
        textfield.bg_color = Some(*constants::colors::CHAT_PANE_FILL_COLOR);

        pane_chatbox.add(chatbox)?;
        pane_chatbox.add(textfield)?;
        layer_ingame.add(pane_chatbox);

        ////// Main Menu ///////
        // Create the main menu pane and add buttons to it
        // TODO: this should be a grid layout so that the x and y positions of the buttons are
        // calculated automatically.
        let mut layer_mainmenu = Layer::new(MAINMENU_LAYER1);
        let (pane_x, pane_y) = (20.0, 20.0);
        let pad = constants::MENU_PADDING;
        let button_width = 180.0;
        let mut pane_mainmenu = Box::new(Pane::new(MAINMENU_PANE1, Rect::new(pane_x, pane_y, 0.0, 0.0)));

        struct ButtonInfo(
            WidgetID,
            UIAction,
            WidgetID,
            &'static str,
        );
        let mainmenu_button_infos: &[ButtonInfo] = &[
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

        let (x, mut y) = (0.0, 0.0);
        let mainmenu_widgets: Vec<Box<dyn Widget>> =
            mainmenu_button_infos.iter().map(|info| -> Box<dyn Widget> {
                let mut button = Box::new(Button::new(ctx, info.0, info.1, info.2, default_font_info, info.3.to_owned()));
                let mut dims = button.rect();
                dims.x = x;
                dims.y = y;
                dims.w = dims.w.max(button_width); // make the buttons an even length (this can only increase the width)
                button.set_rect(dims).unwrap();    // unwrap OK because errors only possible when width or height shrinks
                y += dims.h + pad;
                button
            }).collect();
        pane_mainmenu.add_and_shrink_to_fit(mainmenu_widgets, constants::MENU_PADDING)?;
        layer_mainmenu.add(pane_mainmenu);

        ////// Options ///////
        let mut layer_options = Layer::new(OPTIONS_LAYER1);
        let mut pane_options = Box::new(Pane::new(OPTIONS_PANE1, Rect::new(200.0, 200.0, 0.0, 0.0)));  // XXX fix x,y
        let (mut x, mut y) = (0.0, 0.0);
        let mut options_widgets: Vec<Box<dyn Widget>> = vec![];

        let label_player_name_rect = Rect::new(
            x,
            y,
            constants::MENU_LABEL_WIDTH,
            constants::TEXTFIELD_HEIGHT
        );
        let mut label_player_name = Box::new(
            Label::new(
                ctx,
                OPTIONS_PANE1_LABEL,
                default_font_info,
                "Player Name:".to_owned(),
                *constants::colors::OPTIONS_LABEL_TEXT_COLOR,
                Point2::new(label_player_name_rect.x, label_player_name_rect.y),
            )
        );
        label_player_name.set_rect(label_player_name_rect)?;
        options_widgets.push(label_player_name);
        x += label_player_name_rect.w + constants::MENU_PADDING;    // next widget is to the right of previous

        let tf_player_name_rect = Rect::new(
            x,
            y,
            constants::MENU_INPUT_WIDTH,
            constants::TEXTFIELD_HEIGHT
        );
        let mut tf_player_name = Box::new(TextField::new(OPTIONS_PANE1_TEXTFIELD, default_font_info, tf_player_name_rect));
        tf_player_name.bg_color = Some(*constants::colors::OPTIONS_TEXT_FILL_COLOR);
        options_widgets.push(tf_player_name);

        x = 0.0; // next row
        y += label_player_name_rect.h.max(tf_player_name_rect.h) + constants::MENU_PADDING;
        let (checkbox_width, checkbox_height) = (20.0, 20.0);
        let fullscreen_checkbox = Box::new(
            Checkbox::new(
                ctx,
                OPTIONS_PANE1_FULLSCREEN_CHECKBOX,
                config.get().video.fullscreen,
                default_font_info,
                "Toggle FullScreen".to_owned(),
                Rect::new(x, y, checkbox_width, checkbox_height), //XXX fix x,y
            )
        );
        options_widgets.push(fullscreen_checkbox);

        pane_options.add_and_shrink_to_fit(options_widgets, constants::MENU_PADDING)?;
        layer_options.add(pane_options);

        ////// Add the layers for each screen to the UI layers hash ///////
        ui_layers.insert(Screen::Menu, vec![layer_mainmenu]);
        ui_layers.insert(Screen::Run, vec![layer_ingame]);
        ui_layers.insert(Screen::Options, vec![layer_options]);

        Ok(UILayout {
            layers: ui_layers,
        })
    }
}
