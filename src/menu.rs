
use std::collections::{HashMap};

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum MenuState {
     MenuOff,
     MainMenu,
     Options,
     Video,
     Audio,
     Gameplay,
     Quit
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    text:       String,
    editable:   bool,
    value:      u32,
}

#[derive(Debug)]
pub struct MenuMetaData {
    menuIndex:   u32,
}

pub struct MenuSystem {
    pub    menus:           HashMap<MenuState, Vec<MenuItem> >,
    pub    menu_metadata:   HashMap<MenuState, MenuMetaData>,
    pub    menu_state:      MenuState,
}

impl MenuItem {
    pub fn new(name: String, can_edit: bool, value: u32) -> MenuItem {
        MenuItem {
            text: name,
            editable: can_edit,
            value: value
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }
}

impl MenuSystem {
    pub fn new() -> MenuSystem {
        let mut menu_sys = MenuSystem {
            menus: HashMap::new(),
            menu_metadata:  HashMap::new(),
            menu_state: MenuState::MainMenu,
        };

        
        menu_sys.menus.insert(MenuState::MenuOff,  Vec::new());
        menu_sys.menus.insert(MenuState::MainMenu, Vec::new());
        menu_sys.menus.insert(MenuState::Options,  Vec::new());
        menu_sys.menus.insert(MenuState::Video,    Vec::new());
        menu_sys.menus.insert(MenuState::Audio,    Vec::new());
        menu_sys.menus.insert(MenuState::Gameplay, Vec::new());

        let menu_off    = MenuItem::new(String::from("NULL"), false, 0);
        let start_game  = MenuItem::new(String::from("Start Game"), false, 0);
        let options     = MenuItem::new(String::from("Options"), false, 0);
        let video       = MenuItem::new(String::from("Video"), false, 0);
        let audio       = MenuItem::new(String::from("Audio"), false, 0);
        let gameplay    = MenuItem::new(String::from("Gameplay"), false, 0);
        let quit        = MenuItem::new(String::from("Quit"), false, 0);

        let nothing     = MenuItem::new(String::from("TBD"), true, 100);

        menu_sys.menus
            .get_mut(&MenuState::MenuOff)
            .unwrap()
            .push(menu_off);

        {
            let ref mut main_menu = menu_sys.menus
                .get_mut(&MenuState::MainMenu)
                .unwrap();
            main_menu.push(start_game);
            main_menu.push(options);
            main_menu.push(quit);
        }

        {
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Options)
                .unwrap();
            options_menu.push(video);
            options_menu.push(audio);
            options_menu.push(gameplay);
        }

        {
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Video)
                .unwrap();
            options_menu.push(nothing.clone());
        }

        {
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Audio)
                .unwrap();
            options_menu.push(nothing.clone());
        }

        {
            let ref mut options_menu = menu_sys.menus
                .get_mut(&MenuState::Gameplay)
                .unwrap();
            options_menu.push(nothing.clone());
        }

        menu_sys
    }
}

fn main() {
    let my_menusys = MenuSystem::new();

    // This will print it in arbitrary order
    // Won't matter once we actually are in a State
    for x in my_menusys.menus {
        println!("{:?}, {:?}\n", x.0, x.1);
    }

}
