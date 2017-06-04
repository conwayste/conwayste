
extern crate ggez;

use ggez::graphics::{Point};
use std::collections::{HashMap};

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
// TODO this should be moved into a video/gfx module
pub enum ScreenResolution {
    PX800X600,
    PX1024X768,
    PX1200X960,
    PX1920X1080,
}

#[derive(Debug, Clone)]
pub enum MenuItemValue {
    ValString(String),
    ValI32(i32),
    ValU32(u32),
    ValF32(f32),
    ValBool(bool),
    ValEnum(ScreenResolution),
    ValNone(),
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    id:         MenuItemIdentifier,
    text:       String,
    editable:   bool,
    value:      MenuItemValue,
    coords:     Point
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

pub struct MenuSystem {
    pub    menus:           HashMap<MenuState, Vec<MenuItem> >,
    pub    menu_metadata:   HashMap<MenuState, MenuMetaData>,
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
    pub fn new(identifier: MenuItemIdentifier, name: String, can_edit: bool, value: MenuItemValue, point: Point) -> MenuItem {
        MenuItem {
            id: identifier,
            text: name,
            editable: can_edit,
            value: value,
            coords: point,
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }

    pub fn get_coords(&self) -> &Point {
        &self.coords
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
    pub fn new() -> MenuSystem {
        let mut menu_sys = MenuSystem {
            menus: HashMap::new(),
            menu_metadata:  HashMap::new(),
            menu_state: MenuState::MainMenu,
            controls: MenuControls::new(),
        };

        menu_sys.menus.insert(MenuState::MenuOff,  Vec::new());
        menu_sys.menus.insert(MenuState::MainMenu, Vec::new());
        menu_sys.menus.insert(MenuState::Options,  Vec::new());
        menu_sys.menus.insert(MenuState::Video,    Vec::new());
        menu_sys.menus.insert(MenuState::Audio,    Vec::new());
        menu_sys.menus.insert(MenuState::Gameplay, Vec::new());

        let menu_off    = MenuItem::new(MenuItemIdentifier::None, String::from("NULL"),       false, MenuItemValue::ValU32(0), Point::new(0, 0));
        let start_game  = MenuItem::new(MenuItemIdentifier::StartGame, String::from("Start Game"), false, MenuItemValue::ValU32(0), Point::new(100, 100));
        let options     = MenuItem::new(MenuItemIdentifier::Options, String::from("Options"),    false, MenuItemValue::ValU32(0), Point::new(100, 300));
        let video       = MenuItem::new(MenuItemIdentifier::VideoSettings, String::from("Video"),      false, MenuItemValue::ValU32(0), Point::new(100, 100));
        let audio       = MenuItem::new(MenuItemIdentifier::AudioSettings, String::from("Audio"),      false, MenuItemValue::ValU32(0), Point::new(100, 300));
        let gameplay    = MenuItem::new(MenuItemIdentifier::GameplaySettings, String::from("Gameplay"),   false, MenuItemValue::ValU32(0), Point::new(100, 500));
        let goback      = MenuItem::new(MenuItemIdentifier::ReturnToPreviousMenu, String::from("Back"), false, MenuItemValue::ValU32(0), Point::new(100, 700));
        let quit        = MenuItem::new(MenuItemIdentifier::ExitGame, String::from("Quit"),       false, MenuItemValue::ValU32(0), Point::new(100, 500));
        let nothing     = MenuItem::new(MenuItemIdentifier::None, String::from("TBD"),        false, MenuItemValue::ValNone(), Point::new(0, 0));

        let fullscreen  = MenuItem::new(MenuItemIdentifier::Fullscreen, String::from("Fullscreen:"), true, MenuItemValue::ValBool(false), Point::new(100, 100));
        let resolution  = MenuItem::new(MenuItemIdentifier::Resolution, String::from("Resolution:"), true, MenuItemValue::ValEnum(ScreenResolution::PX1200X960), Point::new(100, 150));

        menu_sys.menus
            .get_mut(&MenuState::MenuOff)
            .unwrap()
            .push(menu_off);
        menu_sys.menu_metadata.insert(MenuState::MenuOff,  MenuMetaData::new(0, 0));

        {
            /////////////////////////
            // Main Menu
            /////////////////////////
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut main_menu = menu_sys.menus
                .get_mut(&MenuState::MainMenu)
                .unwrap();
            main_menu.push(start_game); // 0
            main_menu.push(options);    // 1
            main_menu.push(quit);       // 2

            let count = main_menu.len() as u32;

            metadata.insert(MenuState::MainMenu, MenuMetaData::new(0, count));
        }

        {
            /////////////////////////
            // Options
            /////////////////////////
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Options)
                .unwrap();
            options_menu.push(video);
            options_menu.push(audio);
            options_menu.push(gameplay);
            options_menu.push(goback.clone());

            let count = options_menu.len() as u32;

            metadata.insert(MenuState::Options,  MenuMetaData::new(0, count));
        }

        {
            /////////////////////////
            // Video
            /////////////////////////
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut video_menu = menu_sys.menus
                .get_mut(&MenuState::Video)
                .unwrap();
            video_menu.push(fullscreen);
            video_menu.push(resolution);
            video_menu.push(goback.clone());

            let count = video_menu.len() as u32;
            metadata.insert(MenuState::Video,  MenuMetaData::new(0, count));
        }

        {
            /////////////////////////
            // Audio
            /////////////////////////
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut audio_menu = menu_sys.menus
                .get_mut(&MenuState::Audio)
                .unwrap();
            audio_menu.push(goback.clone());

            let count = audio_menu.len() as u32;
            metadata.insert(MenuState::Audio,  MenuMetaData::new(0, count));
        }

        {
            /////////////////////////
            // Gameplay
            /////////////////////////
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut gameplay_menu = menu_sys.menus
                .get_mut(&MenuState::Gameplay)
                .unwrap();
            gameplay_menu.push(goback.clone());

            let count = gameplay_menu.len() as u32;
            metadata.insert(MenuState::Gameplay,  MenuMetaData::new(0, count));
        }

        menu_sys
    }

    pub fn get_meta_data(&mut self, state: &MenuState) -> &mut MenuMetaData {
        self.menu_metadata.get_mut(&state).unwrap()
    }

    pub fn get_controls(&mut self) -> &mut MenuControls {
        &mut self.controls
    }

    pub fn get_menu_item_list(&mut self, menu_state: &MenuState) -> &mut Vec<MenuItem> {
        self.menus.get_mut(menu_state).unwrap()
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
