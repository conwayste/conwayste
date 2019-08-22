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
use chromatica::css;

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam, Text, Scale};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use crate::config::Config;
use crate::ui::{
    Widget,
    Button,
    Checkbox, ToggleState,
    Chatbox,
    Layer,
    Pane,
    Screen,
    TextField, TextInputState,
    UIAction,
    WidgetID,
};

pub struct UIManager {
    layers: HashMap<Screen, Vec<Layer>>,
}

impl UIManager {
    pub fn new(ctx: &mut Context, config: &Config) -> Self {
        let font = graphics::Font::default(); // Provides DejaVuSerif.ttf
        let mut ui_layers = HashMap::new();

        let chatrect = Rect::new(30.0, 600.0, 300.0, 150.0);
        let mut chatpane = Box::new(Pane::new(WidgetID::InGamePane1, chatrect));
        let mut chatbox = Chatbox::new(WidgetID::InGamePane1Chatbox, 5);
        chatbox.set_size(Rect::new(0.0, 0.0, 300.0, 150.0));
        let chatbox = Box::new(chatbox);
        let chatfield = Box::new(TextField::new( (chatrect.x, chatrect.h), WidgetID::InGamePane1ChatboxTextField));

        chatpane.add(chatbox);
        chatpane.add(chatfield);

        let checkbox = Box::new(Checkbox::new(ctx, &font,
            "Toggle FullScreen",
            Rect::new(10.0, 210.0, 20.0, 20.0),
            WidgetID::MainMenuTestCheckbox,
            UIAction::Toggle( if config.get().video.fullscreen { ToggleState::Enabled } else { ToggleState::Disabled } ),
        ));


        let mut layer_mainmenu = Layer::new(WidgetID::MainMenuLayer1);
        let mut layer_ingame = Layer::new(WidgetID::InGameLayer1);

        // Create a new pane, and add two test buttons to it. Actions do not really matter for now, WIP
        let mut pane = Box::new(Pane::new(WidgetID::MainMenuPane1, Rect::new_i32(20, 20, 300, 250)));
        let mut pane_button = Box::new(Button::new(ctx, &font, "ServerList", WidgetID::MainMenuPane1ButtonYes, UIAction::ScreenTransition(Screen::ServerList)));
        pane_button.set_size(Rect::new(10.0, 10.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, &font, "InRoom", WidgetID::MainMenuPane1ButtonNo, UIAction::ScreenTransition(Screen::InRoom)));
        pane_button.set_size(Rect::new(10.0, 70.0, 180.0, 50.0));
        pane.add(pane_button);

        let mut pane_button = Box::new(Button::new(ctx, &font, "StartGame", WidgetID::MainMenuTestButton, UIAction::ScreenTransition(Screen::Run)));
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