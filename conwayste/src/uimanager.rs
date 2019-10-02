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

use crate::Screen;
use crate::uilayout::UILayout;
use crate::ui::{
    Chatbox,
    Layer,
    TextField,
    WidgetID,
};

pub struct LayoutManager;

impl LayoutManager {
    /// Get the current screen's focused Textfield. This is expected to be on the top-most layer
    pub fn focused_textfield_mut(ui: &mut UILayout, screen: Screen) -> Option<&mut TextField> {
        if let Some(layer) = Self::get_top_layer_from_screen(ui, screen) {
            if let Some(id) = layer.focused_widget {
                let widget = layer.get_widget_mut(id);
                return widget.downcast_mut::<TextField>();
            }
        }
        None
    }

    /// Get the current screen's top most layer
    pub fn get_top_layer_from_screen(ui: &mut UILayout, screen: Screen) -> Option<&mut Layer> {
        if let Some(vec_layer) = ui.layers.get_mut(&screen) {
            return vec_layer.last_mut();
        }
        None
    }

    /// Get all layers associated with the specified Screen
    pub fn get_screen_layers(ui: &mut UILayout, screen:Screen) -> Option<&mut Vec<Layer>> {
        ui.layers.get_mut(&screen)
    }

    /// Retrieve a TextField from its widget ID for the provided Screen
    pub fn textfield_from_id(ui: &mut UILayout, screen: Screen, id: WidgetID) -> Option<&mut TextField> {
        if let Some(layer) = Self::get_top_layer_from_screen(ui, screen) {
            // assumes ID provided is part of the top layer!
            return layer.textfield_from_id(id);
        }
        None
    }

    /// Retreive a Chatbox from its widget ID
    pub fn chatbox_from_id(ui: &mut UILayout, id: WidgetID) -> Option<&mut Chatbox> {
        if let Some(layers) = ui.layers.get_mut(&Screen::Run) {
            if let Some(first_layer) = layers.first_mut() {
                return first_layer.chatbox_from_id(id);
            }
        }
        None
    }
}
