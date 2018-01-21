
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
extern crate toml;

use std::fs::OpenOptions;
use std::io::{Write, Read};
use std::path::Path;

pub const DEFAULT_SCREEN_WIDTH      : f32   = 1200.0;
pub const DEFAULT_SCREEN_HEIGHT     : f32   = 800.0;

// Top-level view of config toml file
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    user:   UserConfig,
    gameplay: GameplayConfig,
    video:  VideoConfig,
    audio:  AudioConfig,
}

// Each section is a sub-structure.
// This will decode from the [user]
// as it is named in that fashion within Config
#[derive(Debug, Deserialize, Serialize, Clone)]
struct UserConfig {
    name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct VideoConfig {
    resolution_x: i32,
    resolution_y: i32,
    fullscreen: bool,

}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AudioConfig {
    master: u8,
    music: u8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GameplayConfig {
    zoom: f32,
}

impl Config {
    pub fn write_default_config(&self) {
        let toml = toml::to_string(&self).unwrap();
        let mut foptions  = OpenOptions::new();
        let mut f = foptions
                    .write(true)
                    .create_new(true)
                    .open("conwayste.toml").unwrap();
        let _ = f.write(toml.as_bytes());
    }
    
    pub fn write_config(&self) {
        let mut foptions  = OpenOptions::new();
        let mut f = foptions
                    .write(true)
                    .open("conwayste.toml").unwrap();
        let toml = toml::to_string(&self).unwrap();
        let _ = f.write(toml.as_bytes());
    }

    pub fn new() -> Self {
        Config {
            user: UserConfig {
                name: String::from("JohnConway"),
            },
            gameplay: GameplayConfig {
                zoom: 5.0f32,
            },
            video: VideoConfig {
                fullscreen: false,
                resolution_x: 0i32,
                resolution_y: 0i32,
            },
            audio: AudioConfig {
                master: 100,
                music: 100,
            },
        }
    }
    
    fn update_config(&mut self, new_config: Config) {
        self.user.name          = new_config.user.name;
        self.gameplay.zoom      = new_config.gameplay.zoom;
        self.video.fullscreen   = new_config.video.fullscreen;
        self.video.resolution_x = new_config.video.resolution_x;
        self.video.resolution_y = new_config.video.resolution_y;
        self.audio.master       = new_config.audio.master;
        self.audio.music        = new_config.audio.music;
    }

    pub fn initialize(&mut self) {
        if Path::exists(Path::new("conwayste.toml")) 
        {
            let mut toml = String::new();
            {
                let mut foptions  = OpenOptions::new();
                let mut f = foptions
                        .read(true)
                        .open("conwayste.toml").unwrap();
                f.read_to_string(&mut toml).unwrap();
            }

            let toml_str = &toml.as_str();
            let config : Config = toml::from_str(toml_str).unwrap();

            self.update_config(config);
        } else {
            self.write_default_config();
        };
    }
}

pub struct ConfigFile {
    settings: Config,
    dirty:    bool,
}

impl ConfigFile {

    pub fn new() -> ConfigFile {
        let mut config = Config::new();
        config.initialize();
        
        ConfigFile {
            settings: config,
            dirty: false,
        }
    }

    pub fn print_to_screen(&self) {
        println!("{:#?}\nDirty:{}", self.settings, self.dirty);
    }

    fn set_dirty(&mut self) {
        self.dirty = true;
    }

    fn set_clean(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty == true
    }

    pub fn write(&mut self) {
        self.settings.write_config();
        self.set_clean();
    }

    pub fn _get_resolution(&self) -> (i32, i32) {
        (self.settings.video.resolution_x, self.settings.video.resolution_y)
    }

    pub fn set_resolution(&mut self, width: i32, height: i32) {
        self.settings.video.resolution_x = width;
        self.settings.video.resolution_y = height;
        self.set_dirty();
    }

    pub fn _is_fullscreen(&self) -> bool {
        self.settings.video.fullscreen == true
    }

    pub fn set_fullscreen(&mut self, is_fullscreen: bool) {
        self.settings.video.fullscreen = is_fullscreen;
        self.set_dirty();
    }

    /*
     *
    pub fn set_master_sound_level(&mut self, level: u8) {
        self.settings.audio.master = level;
        self.set_dirty();
    }

    pub fn set_music_level(&mut self, level: u8) {
        self.settings.audio.music = level;
        self.set_dirty();
    }

     *
     * TODO once we have audio implemented
     *

    pub fn get_master_sound_level(&self) -> u8 {
        self.settings.audio.master
    }

    pub fn get_music_level(&mut self) -> u8 {
        self.settings.audio.music
    }
    */

    pub fn set_zoom_level(&mut self, level: f32) {
        self.settings.gameplay.zoom = level;
        self.set_dirty();
    }

    pub fn get_zoom_level(&self) -> f32 {
        self.settings.gameplay.zoom
    }

/*
    pub fn get_player_name(&self) -> String {
        self.settings.user.name.clone()
    }

    pub fn set_player_name(&mut self, name: String) {
        self.settings.user.name = name;
        self.set_dirty();
    }
*/
}
