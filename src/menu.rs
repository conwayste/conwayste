
extern crate ggez;

use ggez::Context;
use ggez::graphics;
use ggez::graphics::{Rect, Point, Color};
use std::collections::{HashMap};
use video;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum MenuState {
     MenuOff,
     MainMenu,
     Options,
     Video,
     Audio,
     Gameplay,
}

#[derive(Debug, Clone)]
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

    pub fn get_menu_item_list(&self) -> &Vec<MenuItem> {
        &self.menu_items
    }

    pub fn get_menu_item_list_mut(&mut self) -> &mut Vec<MenuItem> {
        &mut self.menu_items
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
    
    pub fn get_value(&self) -> &MenuItemValue {
        &self.value
    }

    pub fn set_value(&mut self, new_val: MenuItemValue) {
        self.value = new_val;
    }

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

        let menu_off    = MenuItem::new(MenuItemIdentifier::None, String::from("NULL"),       false, MenuItemValue::ValNone());
        let start_game  = MenuItem::new(MenuItemIdentifier::StartGame, String::from("Start Game"), false, MenuItemValue::ValNone());
        let options     = MenuItem::new(MenuItemIdentifier::Options, String::from("Options"),    false, MenuItemValue::ValNone());
        let video       = MenuItem::new(MenuItemIdentifier::VideoSettings, String::from("Video"),      false, MenuItemValue::ValNone());
        let audio       = MenuItem::new(MenuItemIdentifier::AudioSettings, String::from("Audio"),      false, MenuItemValue::ValNone());
        let gameplay    = MenuItem::new(MenuItemIdentifier::GameplaySettings, String::from("Gameplay"),   false, MenuItemValue::ValNone());
        let goback      = MenuItem::new(MenuItemIdentifier::ReturnToPreviousMenu, String::from("Back"), false, MenuItemValue::ValNone());
        let quit        = MenuItem::new(MenuItemIdentifier::ExitGame, String::from("Quit"),       false, MenuItemValue::ValNone());

        let fullscreen  = MenuItem::new(MenuItemIdentifier::Fullscreen, String::from("Fullscreen:"), true, MenuItemValue::ValBool(false));
        let resolution  = MenuItem::new(MenuItemIdentifier::Resolution, String::from("Resolution:"), true, MenuItemValue::ValNone());

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

    fn draw_general_menu_view(&mut self,  ctx: &mut Context, index: &i32, cur_menu_state: &MenuState) {
        /// Menu Navigation 
        /////////////////////////////////////////
        match self.menu_state {
             MenuState::MainMenu | MenuState::Options | MenuState::Audio | MenuState::Gameplay | MenuState::Video => {

                /// Draw all menu Items
                ////////////////////////////////////////////////
                {
                    let container = self.menus.get(cur_menu_state).unwrap();
                    let pos_x = container.get_anchor().x();
                    let mut pos_y = container.get_anchor().y();

                    /// Print Menu Items
                    //////////////////////////////////////////////////////
                    for menu_item in container.get_menu_item_list().iter() {
                        let menu_option_string = menu_item.get_text();
                        let menu_option_str = menu_option_string.as_str();
                        let mut menu_option_text = graphics::Text::new(ctx,
                                                               &menu_option_str,
                                                               &self.menu_font).unwrap();

                        let dst = Rect::new(pos_x, pos_y, menu_option_text.width(), menu_option_text.height());
                        graphics::draw(ctx, &mut menu_option_text, None, Some(dst));

                        //pos_x += 50;
                        pos_y += 50;
                    }
                }

                /// Print Current Selection
                ////////////////////////////////////////////////////
                {
                    let cur_option_str = " >";
                    let mut cur_option_text = graphics::Text::new(ctx, &cur_option_str, &self.menu_font).unwrap();

                    let ref container = self.menus.get(&cur_menu_state).unwrap();
                    let coords = container.get_anchor();

                    let dst = Rect::new(coords.x() - 50, coords.y() + 50*index, cur_option_text.width(), cur_option_text.height());
                    graphics::draw(ctx, &mut cur_option_text, None, Some(dst));

                }
            }
            MenuState::MenuOff => {}
        }
    }

    fn draw_specific_menu_view(&mut self, video_settings: &video::VideoSettings, ctx: &mut Context, index: &i32, cur_menu_state: &MenuState) {
        match self.menu_state {
            ////////////////////////////////////
            /// V I D E O
            ///////////////////////////////////
            MenuState::Video => {
                let ref container = self.menus.get(&MenuState::Video).unwrap();
                let anchor = container.get_anchor();

                ///////////////////////////////
                //// Fullscreen
                ///////////////////////////////
                {
                    let coords = (anchor.x() + 200, anchor.y());

                    let is_fullscreen_str = if video_settings.is_fullscreen { "Yes" } else { "No" };
                    let mut is_fullscreen_text = graphics::Text::new(ctx, &is_fullscreen_str, &self.menu_font).unwrap();

                    let dst = Rect::new(coords.0, coords.1, is_fullscreen_text.width(), is_fullscreen_text.height());
                    graphics::draw(ctx, &mut is_fullscreen_text, None, Some(dst));
                }

                ////////////////////////////////
                //// Resolution
                ///////////////////////////////
                {
                    let coords = (anchor.x() + 200, anchor.y() + 50);

                    //let cur_resolution = container.get_menu_item_list().get(1).unwrap().get_value().clone();
                    let (width, height) = video_settings.get_active_resolution();

                    let cur_res_str = format!("{}x{}", width, height);

                    let mut cur_res_text = graphics::Text::new(ctx, &cur_res_str, &self.menu_font).unwrap();
                    let dst = Rect::new(coords.0, coords.1, cur_res_text.width(), cur_res_text.height());
                     graphics::draw(ctx, &mut cur_res_text, None, Some(dst));
                }
            }
             _  => {}
        }
    }

    pub fn draw_menu(&mut self, video_settings: &video::VideoSettings, ctx: &mut Context) {
        let ref cur_menu_state = { self.menu_state.clone() };
        let index = {
            let ref menu_meta = self.get_menu_container(&cur_menu_state).get_metadata();
            menu_meta.get_index() as i32
        };

        self.draw_general_menu_view(ctx, &index, cur_menu_state);
        self.draw_specific_menu_view(video_settings, ctx, &index, cur_menu_state);

    }


}

