/*  Copyright 2017-2018 the Conwayste Developers.
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

use ggez::{Context, GameResult};
use ggez::graphics;
use ggez::graphics::Color;
use ggez::nalgebra::Point2;
use std::collections::HashMap;

use crate::video;
use crate::utils;
use crate::constants::{DEFAULT_ACTIVE_COLOR, DEFAULT_INACTIVE_COLOR};

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum MenuState {
     MainMenu,
     Options,
     Video,
     Audio,
     Gameplay,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum MenuItemIdentifier {
    StartGame,
    Options,
    Connect,
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
    pub id:   MenuItemIdentifier,
    text:     String,
    editable: bool,
    value:    MenuItemValue,
}

#[derive(Debug, Clone)]
pub struct MenuMetaData {
    menu_index:  usize,
    menu_size:   usize,
}

#[derive(Debug)]
pub struct MenuControls {
    dir_key_pressed: bool,
}



#[derive(Debug, Clone)]
pub struct MenuContainer {
    anchor:     Point2<f32>,
    text_width: f32,            // maximum width in pixels of any option
    menu_items: Vec<MenuItem>,
    metadata:   MenuMetaData,
    bg_color:   Color,
    fg_color:   Color,
}

impl MenuContainer {
    pub fn new(x: f32, y: f32) -> MenuContainer {
        MenuContainer {
            anchor: Point2::new(x, y),
            text_width: 0.0,
            menu_items: Vec::<MenuItem>::new(),
            metadata: MenuMetaData::new(0, 0),
            bg_color: Color::new(1.0, 1.0, 1.0, 1.0),
            fg_color: Color::new(0.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn update_menu_items(&mut self, menu_item_list: Vec<MenuItem>) {
        self.menu_items = menu_item_list;
    }

    pub fn update_menu_index(&mut self, index: usize) {
        self.metadata.menu_index = index;
    }

    pub fn update_menu_size(&mut self, size: usize) {
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

    pub fn get_menu_item_index(&self) -> usize {
        self.metadata.menu_index
    }

    pub fn get_anchor(&self) -> Point2<f32> {
        self.anchor
    }

    pub fn get_metadata(&mut self) -> &mut MenuMetaData {
        &mut self.metadata
    }

}

pub struct MenuSystem {
    pub    menus:          HashMap<MenuState, MenuContainer >,
    pub    menu_state:     MenuState,
           controls:       MenuControls,
           font:           graphics::Font,
           inactive_color: graphics::Color,
           active_color:   graphics::Color,
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
    pub fn new(index: usize, size: usize) -> MenuMetaData {
        MenuMetaData {
            menu_index: index,
            menu_size: size,
        }
    }

    pub fn adjust_index(&mut self, amt: isize) {
        let size = self.menu_size;
        let mut new_index = ((self.menu_index as isize + amt) % (size as isize)) as usize;

        if amt < 0 && self.menu_index == 0 {
            new_index = size-1;
        }

        self.menu_index = new_index as usize;
    }
}

impl MenuSystem {
    pub fn new(font: graphics::Font) -> MenuSystem {
        let mut menu_sys = MenuSystem {
            menus:          HashMap::new(),
            menu_state:     MenuState::MainMenu,
            controls:       MenuControls::new(),
            font,
            inactive_color: DEFAULT_INACTIVE_COLOR,
            active_color:   DEFAULT_ACTIVE_COLOR,
        };

        menu_sys.menus.insert(MenuState::MainMenu, MenuContainer::new(400.0, 300.0));
        menu_sys.menus.insert(MenuState::Options,  MenuContainer::new(400.0, 300.0));
        menu_sys.menus.insert(MenuState::Video,    MenuContainer::new(200.0, 100.0));
        menu_sys.menus.insert(MenuState::Audio,    MenuContainer::new(200.0, 100.0));
        menu_sys.menus.insert(MenuState::Gameplay, MenuContainer::new(200.0, 100.0));

        let start_game  = MenuItem::new(MenuItemIdentifier::StartGame,            String ::from("Start Game"), false, MenuItemValue::ValNone());
        let connect     = MenuItem::new(MenuItemIdentifier::Connect,              String ::from("Connect to Server"), false, MenuItemValue::ValNone());
        let options     = MenuItem::new(MenuItemIdentifier::Options,              String ::from("Options"),    false, MenuItemValue::ValNone());
        let video       = MenuItem::new(MenuItemIdentifier::VideoSettings,        String ::from("Video"),      false, MenuItemValue::ValNone());
        let audio       = MenuItem::new(MenuItemIdentifier::AudioSettings,        String ::from("Audio"),      false, MenuItemValue::ValNone());
        let gameplay    = MenuItem::new(MenuItemIdentifier::GameplaySettings,     String ::from("Gameplay"),   false, MenuItemValue::ValNone());
        let goback      = MenuItem::new(MenuItemIdentifier::ReturnToPreviousMenu, String ::from("Back"),       false, MenuItemValue::ValNone());
        let quit        = MenuItem::new(MenuItemIdentifier::ExitGame,             String ::from("Quit"),       false, MenuItemValue::ValNone());

        let fullscreen  = MenuItem::new(MenuItemIdentifier::Fullscreen,           String ::from("Fullscreen:"), true, MenuItemValue::ValBool(false));
        let resolution  = MenuItem::new(MenuItemIdentifier::Resolution,           String ::from("Resolution:"), true, MenuItemValue::ValNone());

        {
            /////////////////////////
            // Main Menu
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::MainMenu).unwrap();

            container.update_menu_items(vec![start_game, connect, options, quit]);
            let count = container.get_menu_item_list().len();
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Options
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Options).unwrap();

            container.update_menu_items(vec![video, audio, gameplay, goback.clone()]);
            let count = container.get_menu_item_list().len();
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Video
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Video).unwrap();

            container.update_menu_items(vec![fullscreen, resolution, goback.clone()]);
            let count = container.get_menu_item_list().len();
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Audio
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Audio).unwrap();

            container.update_menu_items(vec![goback.clone()]);
            let count = container.get_menu_item_list().len();
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        {
            /////////////////////////
            // Gameplay
            /////////////////////////
            let container = menu_sys.menus.get_mut(&MenuState::Gameplay).unwrap();

            container.update_menu_items(vec![goback.clone()]);
            let count = container.get_menu_item_list().len();
            container.update_menu_size(count);
            container.update_menu_index(0);
        }

        menu_sys
    }

    pub fn get_menu_container_mut(&mut self) -> &mut MenuContainer {
        self.menus.get_mut(&mut self.menu_state).unwrap()
    }

    pub fn get_menu_container(&self) -> &MenuContainer {
        self.menus.get(&self.menu_state).unwrap()
    }

    pub fn get_controls(&mut self) -> &mut MenuControls {
        &mut self.controls
    }

    fn draw_general_menu_view(&mut self, _ctx: &mut Context, has_game_started: bool) -> GameResult<()> {
        let index = self.get_menu_container().get_menu_item_index(); // current position in this menu
        // Menu Navigation
        /////////////////////////////////////////
        //TODO: is this match necessary still?
        match self.menu_state {
            MenuState::MainMenu | MenuState::Options | MenuState::Audio | MenuState::Gameplay | MenuState::Video => {

                // Draw all menu Items
                ////////////////////////////////////////////////
                {
                    let container = self.menus.get_mut(&self.menu_state).unwrap();
                    let coords = container.get_anchor();
                    let mut offset = Point2::new(0.0,0.0);

                    let mut max_text_width = container.text_width;
                    for (i, menu_item) in container.get_menu_item_list().iter().enumerate() {
                        let mut menu_option_str: &str = &menu_item.text;

                        if menu_item.id == MenuItemIdentifier::StartGame && has_game_started {
                            menu_option_str = "Resume Game";
                        }

                        let color = if index == i { self.active_color } else { self.inactive_color };
                        let (w, h) = utils::Graphics::draw_text(_ctx, &self.font, color, &menu_option_str,
                                                                 &coords, Some(&offset))?;
                        if max_text_width < w as f32 {
                            max_text_width = w as f32;
                        }

                        offset = utils::Graphics::point_offset(offset, 0.0, h as f32 + 10.0);
                    }
                    if container.text_width < max_text_width {
                        container.text_width = max_text_width;
                    }
                }

                /*
                // Denote Current Selection
                ////////////////////////////////////////////////////
                {
                    let cur_option_str = " >";
                    let ref container = self.menus.get(&self.menu_state).unwrap();
                    let coords = container.get_anchor();
                    let offset = Point2::new(-50.0, (*index) as f32 * 50.0);

                    utils::Graphics::draw_text(_ctx, &self.font, self.active_color, &cur_option_str, &coords, Some(&offset))?;
                }
                */
            }
        }
        Ok(())
    }

    fn draw_specific_menu_view(&mut self, video_settings: &video::VideoSettings, _ctx: &mut Context) -> GameResult<()> {
        match self.menu_state {
            ////////////////////////////////////
            // V I D E O
            ///////////////////////////////////
            MenuState::Video => {
                let ref container = self.menus.get(&MenuState::Video).unwrap();
                let anchor = container.get_anchor();
                let x = anchor.x + container.text_width + 10.0;
                let mut y = anchor.y;

                ///////////////////////////////
                // Fullscreen
                ///////////////////////////////
                let coords = Point2::new(x, y);
                let is_fullscreen_str = if video_settings.is_fullscreen { "Yes" } else { "No" };

                // TODO: color
                let (_w, h) = utils::Graphics::draw_text(_ctx, &self.font, self.inactive_color,
                                                         &is_fullscreen_str, &coords, None)?;
                y += h as f32 + 10.0;

                ////////////////////////////////
                // Resolution
                ///////////////////////////////
                let coords = Point2::new(x, y);
                let (width, height) = video_settings.get_active_resolution();
                let cur_res_str = format!("{}x{}", width, height);

                // TODO: color
                utils::Graphics::draw_text(_ctx, &self.font, self.inactive_color, &cur_res_str,
                                           &coords, None)?;
            }
             _  => {}
        }
        Ok(())
    }

    pub fn draw_menu(&mut self, video_settings: &video::VideoSettings, _ctx: &mut Context, has_game_started: bool) -> GameResult<()> {
        self.draw_general_menu_view(_ctx, has_game_started)?;
        self.draw_specific_menu_view(video_settings, _ctx)?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.menu_state = MenuState::MainMenu;
    }
}

