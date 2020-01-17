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
    Layering,
    Pane,
    TextField,
    UIError,
    UIResult,
    WidgetID,
};

// When adding support for a new widget, use this macro to define a routine which allows the
// developer to search in a `UILayout`/`Screen` pair for a widget by its ID
macro_rules! add_layering_support {
    ($type:ident) => {

        impl $type {
            pub fn widget_from_screen_and_id(
                ui: &mut UILayout,
                screen: Screen,
                id: WidgetID
            ) -> UIResult<&mut $type> {
                if let Some(layer) = LayoutManager::get_screen_layering(ui, screen) {
                    return $type::widget_from_id(layer, id);
                }
                Err(Box::new(UIError::InvalidArgument {
                    reason: format!("{:?} not found in UI Layout", screen)
                }))
            }
        }
    }
}

pub struct LayoutManager;

/// `LayoutManager` is the interface in which UI elements are accessed through using a `UILayout`.
impl LayoutManager {
    /// Get all layers associated with the specified Screen
    pub fn get_screen_layering(ui: &mut UILayout, screen:Screen) -> Option<&mut Layering> {
        ui.layers.get_mut(&screen)
    }

    /// Get the current screen's focused Textfield. This is expected to be on the top-most layer
    pub fn focused_textfield_mut(ui: &mut UILayout, screen: Screen) -> UIResult<&mut TextField> {
        if let Some(layer) = Self::get_screen_layering(ui, screen) {
            if let Some(id) = layer.focused_widget_id() {
                return TextField::widget_from_id(layer, id);
            }
        }
        Err(Box::new(UIError::WidgetNotFound {
            reason: format!("Layering for screen {:?} does not have a TextField in focus", screen)
        }))
    }
}

add_layering_support!(Button);
add_layering_support!(Checkbox);
add_layering_support!(Label);
add_layering_support!(Pane);
add_layering_support!(TextField);
add_layering_support!(Chatbox);
