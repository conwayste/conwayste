
extern crate ggez;

use ggez::graphics::{Point, Color};
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
    ValEnum(video::ScreenResolution),
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

    /*
    pub fn get_video_menu_current_resolution(cur_resolution: MenuItemValue) -> Option<String> {
        match cur_resolution {
            MenuItemValue::ValEnum(x) => { Some(String::from(video::get_resolution_str(x))) }
            _ => { None }
        }
    }

    pub fn set_next_menu_resolution(video_menu: &mut Vec<MenuItem>) -> (u32, u32) {

        let mut screen_res_item = video_menu.get_mut(1).unwrap();
        let cur_resolution = screen_res_item.get_value().clone();

        let resolution = match cur_resolution {
            MenuItemValue::ValEnum(x) => 
            {
                match x {
                    video::ScreenResolution::PX800X600 => {
                        screen_res_item.set_value(MenuItemValue::ValEnum(video::ScreenResolution::PX1024X768));
                        (1024, 768)
                    }
                    video::ScreenResolution::PX1024X768 => {
                        screen_res_item.set_value(MenuItemValue::ValEnum(video::ScreenResolution::PX1200X960));
                        (1200, 960)
                    }
                    video::ScreenResolution::PX1200X960 => {
                        screen_res_item.set_value(MenuItemValue::ValEnum(video::ScreenResolution::PX1920X1080));
                        (1920, 1080)
                    }
                    video::ScreenResolution::PX1920X1080 => {
                        screen_res_item.set_value(MenuItemValue::ValEnum(video::ScreenResolution::PX800X600));
                        (800, 600)
                    }
                }
            }
            _ => {(0,0)}
        };
        resolution
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
    pub fn new() -> MenuSystem {
        let mut menu_sys = MenuSystem {
            menus: HashMap::new(),
            menu_state: MenuState::MainMenu,
            controls: MenuControls::new(),
        };

        // TODO needs to reference the window coordinates
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
        let resolution  = MenuItem::new(MenuItemIdentifier::Resolution, String::from("Resolution:"), true, MenuItemValue::ValEnum(video::ScreenResolution::PX1200X960));

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

}

#[allow(dead_code)]
// when compiled with rustc we'll print out the menus
fn main() {
    let my_menusys = MenuSystem::new();

    // This will print it in arbitrary order
    // Won't matter once we actually are in a State
    for x in my_menusys.menus {
        println!("{:?}, {:?}\n", x.0, x.1);
    }

}
