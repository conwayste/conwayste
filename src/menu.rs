/*  Copyright 2017 the Conwayste Developers.
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

use ggez::Context;
use ggez::graphics;
use ggez::graphics::{Point, Color};
use std::collections::{HashMap};

use video;
use utils;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum MenuState {
     MenuOff,
     MainMenu,
     Options,
     Video,
     Audio,
     Gameplay,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItemIdentifier {
    None,
    StartGame,
    Options,
    AudioSettings,
    VideoSettings,
    GameplaySettings,
    ExitGame,
    ReturnToPreviousMenu,

    Fullscreen,
    Resolution,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum MenuItemValue {
    ValString(String),
    ValI32(i32),
    ValU32(u32),
    ValF32(f32),
    ValBool(bool),
//    ValEnum(),
    ValNone(),
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    id:         MenuItemIdentifier,
    text:       String,
    editable:   bool,
    value:      MenuItemValue,
}

#[derive(Debug, Clone)]
pub struct MenuMetaData {
    menu_index:  u32,
    menu_size:   u32,
}

#[derive(Debug)]
pub struct MenuControls {
    dir_key_pressed:       bool,
}



#[derive(Debug, Clone)]
pub struct MenuContainer {
    anchor:     Point,
    menu_items: Vec<MenuItem>,
    metadata:   MenuMetaData,
    bg_color:   Color,
    fg_color:   Color,
}

impl MenuContainer {
    pub fn new(x: i32, y: i32) -> MenuContainer {
        MenuContainer {
            anchor: Point::new(x, y),
            menu_items: Vec::<MenuItem>::new(),
            metadata: MenuMetaData::new(0, 0),
            bg_color: Color::RGB(100, 100, 100),
            fg_color: Color::RGB(0, 255, 255),
        }
    }

    pub fn update_menu_items(&mut self, menu_item_list: Vec<MenuItem>) {
        self.menu_items = menu_item_list;
    }

    pub fn update_menu_index(&mut self, index: u32) {
        self.metadata.menu_index = index;
    }

    pub fn update_menu_size(&mut self, size: u32) {
        self.metadata.menu_size = size;
    }

    /*
     * May be used if we have toggable state-dependant items
    pub fn get_menu_item_list_mut(&mut self) -> &mut Vec<MenuItem> {
        &mut self.menu_items
    }
    */

    pub fn get_menu_item_list(&self) -> &Vec<MenuItem> {
        &self.menu_items
    }

    pub fn get_anchor(&self) -> Point {
        self.anchor
    }

    pub fn get_metadata(&mut self) -> &mut MenuMetaData {
        &mut self.metadata
    }

}

pub struct MenuSystem {
    pub    menus:           HashMap<MenuState, MenuContainer >,
    pub    menu_state:      MenuState,
           controls:        MenuControls,
           menu_font:       graphics::Font,
}

impl MenuControls {
    pub fn new() -> MenuControls {
        MenuControls {
            dir_key_pressed: false,
        }
    }

    pub fn set_menu_key_pressed(&mut self, state: bool) {
        self.dir_key_pressed = state;
    }

    pub fn is_menu_key_pressed(&self) -> bool {
        self.dir_key_pressed
    }
}

impl MenuItem {
    pub fn new(identifier: MenuItemIdentifier, name: String, can_edit: bool, value: MenuItemValue) -> MenuItem {
        MenuItem {
            id: identifier,
            text: name,
            editable: can_edit,
            value: value,
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }

    pub fn get_id(&self) -> MenuItemIdentifier {
        self.id.clone()
    }
    
    /*
    pub fn get_value(&self) -> &MenuItemValue {
        &self.value
    }
    */

    /*
    pub fn set_value(&mut self, new_val: MenuItemValue) {
        self.value = new_val;
    }
    */

}

impl MenuMetaData {
    pub fn new(index: u32, size: u32) -> MenuMetaData {
        MenuMetaData {
            menu_index: index,
            menu_size: size,
        }
    }

    pub fn get_index(&self) -> usize {
        self.menu_index as usize
    }

    pub fn adjust_index(&mut self, amt: i32) {
        let size = self.menu_size;
        let mut new_index = (self.menu_index as i32 + amt) as u32 % size;

        if amt < 0 && self.menu_index == 0 {
            new_index = size-1;
        }

        self.menu_index = new_index as u32;
    }
}

impl MenuSystem {
    pub fn new(font: graphics::Font) -> MenuSystem {
        let mut menu_sys = MenuSystem {
            menus: HashMap::new(),
            menu_state: MenuState::MainMenu,
            controls: MenuControls::new(),
            menu_font: font,
        };

        menu_sys.menus.insert(MenuState::MenuOff,  MenuContainer::new(0, 0));
        menu_sys.menus.insert(MenuState::MainMenu, MenuContainer::new(400, 300));
        menu_sys.menus.insert(MenuState::Options,  MenuContainer::new(400, 300));
        menu_sys.menus.insert(MenuState::Video,    MenuContainer::new(200, 100));
        menu_sys.menus.insert(MenuState::Audio,    MenuContainer::new(200, 100));
        menu_sys.menus.insert(MenuState::Gameplay, MenuContainer::new(200, 100));

        let menu_off    = MenuItem::new(MenuItemIdentifier::None,                 String ::from("NULL"),       false, MenuItemValue::ValNone());
        let start_game  = MenuItem::new(MenuItemIdentifier::StartGame,            String ::from("Start Game"), false, MenuItemValue::ValNone());
        let options     = MenuItem::new(MenuItemIdentifier::Options,              String ::from("Options"),    false, MenuItemValue::ValNone());
        let video       = MenuItem::new(MenuItemIdentifier::VideoSettings,        String ::from("Video"),      false, MenuItemValue::ValNone());
        let audio       = MenuItem::new(MenuItemIdentifier::AudioSettings,        String ::from("Audio"),      false, MenuItemValue::ValNone());
        let gameplay    = MenuItem::new(MenuItemIdentifier::GameplaySettings,     String ::from("Gameplay"),   false, MenuItemValue::ValNone());
        let goback      = MenuItem::new(MenuItemIdentifier::ReturnToPreviousMenu, String ::from("Back"),       false, MenuItemValue::ValNone());
        let quit        = MenuItem::new(MenuItemIdentifier::ExitGame,             String ::from("Quit"),       false, MenuItemValue::ValNone());

        let fullscreen  = MenuItem::new(MenuItemIdentifier::Fullscreen,           String ::from("Fullscreen:"), true, MenuItemValue::ValBool(false));
        let resolution  = MenuItem::new(MenuItemIdentifier::Resolution,           String ::from("Resolution:"), true, MenuItemValue::ValNone());

        {
            let container = menu_sys.menus.get_mut(&MenuState::MenuOff).unwrap();

            container.update_menu_items(vec![menu_off]);
            container.update_menu_size(1);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Main Menu
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::MainMenu).unwrap();

            container.update_menu_items(vec![start_game, options, quit]);
            let count = container.get_menu_item_list().len() as u32;
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Options
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Options).unwrap();

            container.update_menu_items(vec![video, audio, gameplay, goback.clone()]);
            let count = container.get_menu_item_list().len() as u32;
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Video
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Video).unwrap();

            container.update_menu_items(vec![fullscreen, resolution, goback.clone()]);
            let count = container.get_menu_item_list().len() as u32;
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Audio
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Audio).unwrap();

            container.update_menu_items(vec![goback.clone()]);
            let count = container.get_menu_item_list().len() as u32;
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Gameplay
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Gameplay).unwrap();

            container.update_menu_items(vec![goback.clone()]);
            let count = container.get_menu_item_list().len() as u32;
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        menu_sys
    }

    pub fn get_menu_container(&mut self, state: &MenuState) -> &mut MenuContainer {
        self.menus.get_mut(&state).unwrap()
    }

    pub fn get_controls(&mut self) -> &mut MenuControls {
        &mut self.controls
    }

    fn draw_general_menu_view(&mut self, _ctx: &mut Context, index: &i32, cur_menu_state: &MenuState, has_game_started: bool) {
        // Menu Navigation 
        /////////////////////////////////////////
        match self.menu_state {
             MenuState::MainMenu | MenuState::Options | MenuState::Audio | MenuState::Gameplay | MenuState::Video => {

                // Draw all menu Items
                ////////////////////////////////////////////////
                {
                    let container = self.menus.get(cur_menu_state).unwrap();
                    let coords = container.get_anchor();
                    let mut offset = Point::new(0,0);

                    for menu_item in container.get_menu_item_list().iter() {
                        let menu_option_string = menu_item.get_text();
                        let mut menu_option_str = menu_option_string.as_str();

                        if menu_item.get_id() == MenuItemIdentifier::StartGame && has_game_started {
                            menu_option_str = "Resume Game";
                        }

                        utils::Graphics::draw_text(_ctx, &self.menu_font, &menu_option_str, &coords, Some(&offset));

                        offset = offset.offset(0, 50);
                    }
                }

                // Denote Current Selection
                ////////////////////////////////////////////////////
                {
                    let cur_option_str = " >";
                    let ref container = self.menus.get(&cur_menu_state).unwrap();
                    let coords = container.get_anchor();
                    let offset = Point::new(-50, index*50);

                    utils::Graphics::draw_text(_ctx, &self.menu_font, &cur_option_str, &coords, Some(&offset));
                }
            }
            MenuState::MenuOff => {}
        }
    }

    fn draw_specific_menu_view(&mut self, video_settings: &video::VideoSettings, _ctx: &mut Context) {
        match self.menu_state {
            ////////////////////////////////////
            //// V I D E O
            ///////////////////////////////////
            MenuState::Video => {
                let ref container = self.menus.get(&MenuState::Video).unwrap();
                let anchor = container.get_anchor();

                ///////////////////////////////
                //// Fullscreen
                ///////////////////////////////
                {
                    let coords = Point::new(anchor.x() + 200, anchor.y());
                    let is_fullscreen_str = if video_settings.is_fullscreen { "Yes" } else { "No" };

                    utils::Graphics::draw_text(_ctx, &self.menu_font, &is_fullscreen_str, &coords, None);
                }

                ////////////////////////////////
                //// Resolution
                ///////////////////////////////
                {
                    let coords = Point::new(anchor.x() + 200, anchor.y() + 50);
                    let (width, height) = video_settings.get_active_resolution();
                    let cur_res_str = format!("{}x{}", width, height);

                    utils::Graphics::draw_text(_ctx, &self.menu_font, &cur_res_str, &coords, None);
               }
            }
             _  => {}
        }
    }

    pub fn draw_menu(&mut self, video_settings: &video::VideoSettings, _ctx: &mut Context, has_game_started: bool) {
        let ref cur_menu_state = { self.menu_state.clone() };
        let index = {
            let ref menu_meta = self.get_menu_container(&cur_menu_state).get_metadata();
            menu_meta.get_index() as i32
        };

        self.draw_general_menu_view(_ctx, &index, cur_menu_state, has_game_started);
        self.draw_specific_menu_view(video_settings, _ctx);
    }
}

