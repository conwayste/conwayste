
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
pub struct MenuItem {
    text:       String,
    editable:   bool,
    value:      u32,
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
    pub fn new(name: String, can_edit: bool, value: u32, point: Point) -> MenuItem {
        MenuItem {
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
}

impl MenuMetaData {
    pub fn new(index: u32, size: u32) -> MenuMetaData {
        MenuMetaData {
            menu_index: index,
            menu_size: size,
        }
    }

    pub fn get_index(&self) -> &u32 {
        &self.menu_index
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

        let menu_off    = MenuItem::new(String::from("NULL"),       false, 0, Point::new(0, 0));
        let start_game  = MenuItem::new(String::from("Start Game"), false, 0, Point::new(100, 100));
        let options     = MenuItem::new(String::from("Options"),    false, 0, Point::new(100, 300));
        let video       = MenuItem::new(String::from("Video"),      false, 0, Point::new(100, 100));
        let audio       = MenuItem::new(String::from("Audio"),      false, 0, Point::new(100, 300));
        let gameplay    = MenuItem::new(String::from("Gameplay"),   false, 0, Point::new(100, 500));
        let quit        = MenuItem::new(String::from("Quit"),       false, 0, Point::new(100, 500));
        let nothing     = MenuItem::new(String::from("TBD"),        true, 100, Point::new(0, 0));

        menu_sys.menus
            .get_mut(&MenuState::MenuOff)
            .unwrap()
            .push(menu_off);
        menu_sys.menu_metadata.insert(MenuState::MenuOff,  MenuMetaData::new(0, 0));

        {
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut main_menu = menu_sys.menus
                .get_mut(&MenuState::MainMenu)
                .unwrap();
            main_menu.push(start_game);
            main_menu.push(options);
            main_menu.push(quit);

            let count = main_menu.len() as u32;

            metadata.insert(MenuState::MainMenu, MenuMetaData::new(0, count));
        }

        {
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Options)
                .unwrap();
            options_menu.push(video);
            options_menu.push(audio);
            options_menu.push(gameplay);

            let count = options_menu.len() as u32;

            metadata.insert(MenuState::Options,  MenuMetaData::new(0, count));
        }

        {
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut video_menu = menu_sys.menus
                .get_mut(&MenuState::Video)
                .unwrap();
            video_menu.push(nothing.clone());

            let count = video_menu.len() as u32;
            metadata.insert(MenuState::Video,  MenuMetaData::new(0, count));
        }

        {
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut audio_menu = menu_sys.menus
                .get_mut(&MenuState::Audio)
                .unwrap();
            audio_menu.push(nothing.clone());

            let count = audio_menu.len() as u32;
            metadata.insert(MenuState::Audio,  MenuMetaData::new(0, count));
        }

        {
            let ref mut metadata = menu_sys.menu_metadata;
            let ref mut gameplay_menu = menu_sys.menus
                .get_mut(&MenuState::Gameplay)
                .unwrap();
            gameplay_menu.push(nothing.clone());

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
