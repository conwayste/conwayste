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
use std::rc::Rc;


use ggez::graphics::{Rect, Font};

use ggez::{Context};

use crate::constants::CHATBOX_INPUT_VISIBILE_END_INDEX;
use crate::config::Config;
use crate::Screen;
use crate::ui::{
    self,
    Widget,
    Button,
    Checkbox, ToggleState,
    Chatbox,
    Layer,
    Pane,
    TextField,
    UIAction,
    WidgetID,
};

pub struct UIManager {
    layers: HashMap<Screen, Vec<Layer>>,
}

impl UIManager {
    // PR_GATE move UI creation out of the manager per PR feedback
    pub fn new(ctx: &mut Context, config: &Config, font: Rc<Font>) -> Self {
        let mut ui_layers = HashMap::new();

        let chat_pane_rect = Rect::new(30.0, 40.0, 300.0, 150.0);
        let mut chatpane = Box::new(Pane::new(ui::INGAME_PANE1, chat_pane_rect));

        const CHATBOX_HISTORY: usize = 5;
        let chatbox_rect = Rect::new(0.0, 0.0, chat_pane_rect.w, chat_pane_rect.h);
        let mut chatbox = Chatbox::new(ui::INGAME_PANE1_CHATBOX,
            Rc::clone(&font),
            CHATBOX_HISTORY
        );
        chatbox.set_size(chatbox_rect);
        let chatbox = Box::new(chatbox);

        const CHAT_TEXTFIELD_HEIGHT: f32 = (20.0 + 5.0);
        let chatfield_rect = Rect::new(chatbox_rect.x, chatbox_rect.bottom(), chatbox_rect.w, CHAT_TEXTFIELD_HEIGHT);
        let chatfield = Box::new(TextField::new(ui::INGAME_PANE1_CHATBOXTEXTFIELD,
            Rc::clone(&font),
            chatfield_rect,
            CHATBOX_INPUT_VISIBILE_END_INDEX
        ));

        chatpane.add(chatbox);
        chatpane.add(chatfield);

        let checkbox = Box::new(Checkbox::new(ctx, ui::MAINMENU_TESTCHECKBOX,
            UIAction::Toggle( if config.get().video.fullscreen { ToggleState::Enabled } else { ToggleState::Disabled } ),
            Rc::clone(&font),
            "Toggle FullScreen".to_owned(),
            Rect::new(10.0, 210.0, 20.0, 20.0),
        ));


        let mut layer_mainmenu = Layer::new(ui::MAINMENU_LAYER1);
        let mut layer_ingame = Layer::new(ui::INGAME_LAYER1);

        // Create a new pane, and add two test buttons to it. Actions do not really matter for now, WIP
        let mut pane = Box::new(Pane::new(ui::MAINMENU_PANE1, Rect::new_i32(20, 20, 300, 250)));
        let mut pane_button = Box::new(Button::new(ctx, ui::MAINMENU_PANE1_BUTTONYES,
            UIAction::ScreenTransition(Screen::ServerList),
            ui::MAINMENU_PANE1_BUTTONYESLABEL,
            Rc::clone(&font),
            "ServerList".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 10.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, ui::MAINMENU_PANE1_BUTTONNO,
            UIAction::ScreenTransition(Screen::InRoom),
            ui::MAINMENU_PANE1_BUTTONNOLABEL,
            Rc::clone(&font),
            "InRoom".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 70.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, ui::MAINMENU_TESTBUTTON,
            UIAction::ScreenTransition(Screen::Run),
            ui::MAINMENU_TESTBUTTONLABEL,
            Rc::clone(&font),
            "StartGame".to_owned()
        ));
        pane_button.set_size(Rect::new(10.0, 130.0, 180.0, 50.0));
        pane.add(pane_button);

        pane.add(checkbox);

        layer_mainmenu.add(pane);
        layer_ingame.add(chatpane);

        ui_layers.insert(Screen::Menu, vec![layer_mainmenu]);
        ui_layers.insert(Screen::Run, vec![layer_ingame]);

        UIManager {
            layers: ui_layers,
        }
    }


    /// Returns a reference to the layer's currently focused text field
    pub fn focused_textfield_mut(&mut self, screen: Screen) -> Option<&mut TextField> {
        if let Some(layer) = self.get_top_layer_from_screen(screen) {
            if let Some(id) = layer.focused_widget {
                let widget = layer.get_widget_mut(id);
                return widget.downcast_mut::<TextField>();
            }
        }
        None
    }

    /// Get the current screen's top most layer
    pub fn get_top_layer_from_screen(&mut self, screen: Screen) -> Option<&mut Layer> {
        if let Some(vec_layer) = self.layers.get_mut(&screen) {
            return vec_layer.last_mut();
        }
        None
    }

    pub fn get_screen_layers(&mut self, screen:Screen) -> Option<&mut Vec<Layer>> {
        self.layers.get_mut(&screen)
    }

    pub fn textfield_from_id(&mut self, screen: Screen, id: WidgetID) -> Option<&mut TextField> {
        if let Some(layer) = self.get_top_layer_from_screen(screen) {
            // assumes ID provided is part of the top layer!
            return layer.textfield_from_id(id);
        }
        None
    }

    pub fn chatbox_from_id(&mut self, id: WidgetID) -> Option<&mut Chatbox> {
        if let Some(layers) = self.layers.get_mut(&Screen::Run) {
            if let Some(first_layer) = layers.first_mut() {
                return first_layer.chatbox_from_id(id);
            }
        }
        None
    }
}
