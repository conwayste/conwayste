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
#![allow(unused)]

use crate::Screen;
use crate::uilayout::UILayout;
use crate::ui::{
    Button,
    Chatbox,
    Checkbox,
    Label,
    Layer,
    Pane,
    TextField,
    WidgetID,
};

// When adding support for a new widget, use this macro to define a routine to search in the
// `UILayout` under the `Screen` for a `WidgetID` of that type
macro_rules! widget_from_screen_and_ui {
    ($type:ident) => {

        impl $type {
            pub fn widget_from_screen_and_id(ui: &mut UILayout, screen: Screen, id: WidgetID) -> Option<&mut $type> {
                if let Some(layer) = LayoutManager::get_top_layer(ui, screen) {
                    // assumes ID provided is part of the top layer!
                    return $type::widget_from_id(layer, id);
                }
                None
            }
        }
    }
}

pub struct LayoutManager;

impl LayoutManager {
    /// Get the current screen's top most layer
    pub fn get_top_layer(ui: &mut UILayout, screen: Screen) -> Option<&mut Layer> {
        if let Some(vec_layer) = ui.layers.get_mut(&screen) {
            return vec_layer.last_mut();
        }
        None
    }

    /// Get all layers associated with the specified Screen
    pub fn get_screen_layers(ui: &mut UILayout, screen:Screen) -> Option<&mut Vec<Layer>> {
        ui.layers.get_mut(&screen)
    }

    /// Get the current screen's focused Textfield. This is expected to be on the top-most layer
    pub fn focused_textfield_mut(ui: &mut UILayout, screen: Screen) -> Option<&mut TextField> {
        if let Some(layer) = Self::get_top_layer(ui, screen) {
            if let Some(id) = layer.focused_widget {
                let widget = layer.get_widget_mut(id);
                return widget.downcast_mut::<TextField>();
            }
        }
        None
    }

    /// Retreive a Chatbox from its widget ID
    pub fn chatbox_from_id(ui: &mut UILayout, id: WidgetID) -> Option<&mut Chatbox> {
        // Not generated using `gen_screen_id_for_widget` as
        // the Chatbox exists at a fixed position in the layers of Screen::Run
        if let Some(layers) = ui.layers.get_mut(&Screen::Run) {
            if let Some(first_layer) = layers.first_mut() {
                return Chatbox::widget_from_id(first_layer, id);
            }
        }
        None
    }
}

widget_from_screen_and_ui!(Button);
widget_from_screen_and_ui!(Checkbox);
widget_from_screen_and_ui!(Label);
widget_from_screen_and_ui!(Pane);
widget_from_screen_and_ui!(TextField);
