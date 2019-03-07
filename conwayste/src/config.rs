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
use crate::constants::{CONFIG_FILE_PATH, DEFAULT_ZOOM_LEVEL};

/// User configuration descriptor which contains all of the configurable settings
/// for this game. These *should* be modified within the game, but one can 
/// always edit this file directly. The game will fail to load if there are
/// any errors parsing the `conwayste.toml` file.
// Top-level view of config toml file
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    user:   UserConfig,
    gameplay: GameplayConfig,
    video:  VideoConfig,
    audio:  AudioConfig,
}

/// This will decode from the [user] section and contains user-specific settings.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct UserConfig {
    name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Graphics-related settings like resolution, fullscreen, and more!
struct VideoConfig {
    resolution_x: i32,
    resolution_y: i32,
    fullscreen: bool,

}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Audio-related settings like sound and music levels.
struct AudioConfig {
    master: u8,
    music: u8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Gameplay-related settings. Pretty empty for now.
struct GameplayConfig {
    zoom: f32,
}

impl Config {
    /// Writes the default `conwayste.toml` configuration file.
    pub fn write_default_config(&self) {
        let toml = toml::to_string(&self).unwrap();
        let mut foptions  = OpenOptions::new();
        let mut f = foptions
                    .write(true)
                    .create_new(true)
                    .open(CONFIG_FILE_PATH).unwrap();
        let _ = f.write(toml.as_bytes());
    }
    
    /// Writes the in-memory config to the `conwayste.toml` configuration file.
    pub fn write_config(&self) {
        let mut foptions  = OpenOptions::new();
        let mut f = foptions
                    .write(true)
                    .open(CONFIG_FILE_PATH).unwrap();
        let toml = toml::to_string(&self).unwrap();
        let _ = f.write(toml.as_bytes());
    }

    /// Creates the default configuration with mostly invalid settings.
    pub fn new() -> Self {
        Config {
            user: UserConfig {
                name: String::from("JohnConway"),
            },
            gameplay: GameplayConfig {
                zoom: DEFAULT_ZOOM_LEVEL,
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
    
    /// Helper to simply copy the local file copy to memory.
    fn update_config(&mut self, new_config: Config) {
        self.user.name          = new_config.user.name;
        self.gameplay.zoom      = new_config.gameplay.zoom;
        self.video.fullscreen   = new_config.video.fullscreen;
        self.video.resolution_x = new_config.video.resolution_x;
        self.video.resolution_y = new_config.video.resolution_y;
        self.audio.master       = new_config.audio.master;
        self.audio.music        = new_config.audio.music;
    }

    /// Initializes the in-memory configuration settings from an 
    /// already existing `conwayste.toml` file or or with the 
    /// default settings. This will fail if the toml file cannot
    /// be parsed correctly.
    pub fn initialize(&mut self) {
        if Path::exists(Path::new(CONFIG_FILE_PATH))
        {
            let mut toml = String::new();
            {
                let mut foptions  = OpenOptions::new();
                let mut f = foptions
                        .read(true)
                        .open(CONFIG_FILE_PATH).unwrap();
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

/// Pretty straightfoward. If the file is `dirty`, then all of the configuration settings
/// will be flushed down to the local file.
pub struct ConfigFile {
    settings: Config,
    dirty:    bool,
}

impl ConfigFile {

    /// Creates a new manager to handle the conwayste configuration settings.
    pub fn new() -> ConfigFile {
        let mut config = Config::new();
        config.initialize();
        
        ConfigFile {
            settings: config,
            dirty: false,
        }
    }

    /// Prints the configuration via `:?` for sanity checking.
    pub fn print_to_screen(&self) {
        println!("{:#?}\nDirty:{}", self.settings, self.dirty);
    }

    /// Sets the configuration file as dirty.
    fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Marks the configuration file as flushed to the disk.
    fn set_clean(&mut self) {
        self.dirty = false;
    }

    /// Queries to see if the configuration file is dirty or not.
    pub fn is_dirty(&self) -> bool {
        self.dirty == true
    }

    /// Writes the configuration file to the local filesystem.
    pub fn write(&mut self) {
        self.settings.write_config();
        self.set_clean();
    }

    /// Gets the configuration files copy of what the resolution is as a tuple.
    pub fn get_resolution(&self) -> (i32, i32) {
        (self.settings.video.resolution_x, self.settings.video.resolution_y)
    }

    /// Updates the resolution within the configuration file.
    pub fn set_resolution(&mut self, width: i32, height: i32) {
        self.settings.video.resolution_x = width;
        self.settings.video.resolution_y = height;
        self.set_dirty();
    }

    /// Checks to see if the game was listed as fullscreen within the toml settings.
    pub fn is_fullscreen(&self) -> bool {
        self.settings.video.fullscreen == true
    }

    /// Sets the fullscreen setting with the providied boolean.
    pub fn set_fullscreen(&mut self, is_fullscreen: bool) {
        self.settings.video.fullscreen = is_fullscreen;
        self.set_dirty();
    }

    /*
     *
    /// Sets the master sound level to the specified value.
    /// Value range is 0 to 100.
    pub fn set_master_sound_level(&mut self, level: u8) {
        self.settings.audio.master = level;
        self.set_dirty();
    }

    /// Sets the music level to the specified value.
    /// Value range is 0 to 100.
    pub fn set_music_level(&mut self, level: u8) {
        self.settings.audio.music = level;
        self.set_dirty();
    }

     *
     * TODO once we have audio implemented
     *

    /// Gets the master sound level.
    pub fn get_master_sound_level(&self) -> u8 {
        self.settings.audio.master
    }

    /// Gets the master sound level to the specified value.
    pub fn get_music_level(&mut self) -> u8 {
        self.settings.audio.music
    }
    */

    /// Sets the zoom level within the configuration file.
    pub fn set_zoom_level(&mut self, level: f32) {
        self.settings.gameplay.zoom = level;
        self.set_dirty();
    }

    /// Gets the zoom level file specified in the toml file.
    pub fn get_zoom_level(&self) -> f32 {
        self.settings.gameplay.zoom
    }

/*
    /// Gets the player name specified within the configuration file.
    pub fn get_player_name(&self) -> String {
        self.settings.user.name.clone()
    }

    /// Sets the player name within the configuration file.
    pub fn set_player_name(&mut self, name: String) {
        self.settings.user.name = name;
        self.set_dirty();
    }
*/
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_init_default_config() {
        let config = Config::new();

        assert_eq!(config.audio.master, 100);
        assert_eq!(config.audio.music, 100);
        assert_eq!(config.video.fullscreen, false);
        assert_eq!(config.video.resolution_x, 0);
        assert_eq!(config.video.resolution_y, 0);
        assert_eq!(config.gameplay.zoom, DEFAULT_ZOOM_LEVEL);
        assert_eq!(config.user.name, "JohnConway");
    }

    #[test]
    fn test_update_config() {
        let mut config = Config::new();
        let mut secondary_config = Config::new();

        secondary_config.user.name = String::from("TestUser");
        secondary_config.audio.master = 50;
        secondary_config.audio.music = 50;
        secondary_config.video.fullscreen = true;
        secondary_config.gameplay.zoom = 1000.0;

        config.update_config(secondary_config);
        assert_eq!(config.audio.master, 50);
        assert_eq!(config.audio.music, 50);
        assert_eq!(config.video.fullscreen, true);
        assert_eq!(config.video.resolution_x, 0);
        assert_eq!(config.video.resolution_y, 0);
        assert_eq!(config.gameplay.zoom, 1000.0);
        assert_eq!(config.user.name, "TestUser");
    }

    #[test]
    fn test_config_file_cleanliness() {
        let mut configfile = ConfigFile::new();

        assert_eq!(configfile.is_dirty(), false);

        configfile.set_dirty();
        assert_eq!(configfile.is_dirty(), true);

        configfile.write();
        assert_eq!(configfile.is_dirty(), false);
    }

    #[test]
    fn test_modify_default_config_and_write() {
        let mut configfile = ConfigFile::new();

        assert_eq!(configfile.is_dirty(), false);

        configfile.set_zoom_level(10.0);
        assert_eq!(configfile.get_zoom_level(), 10.0);
        assert_eq!(configfile.is_dirty(), true);

        configfile.write();
        assert_eq!(configfile.is_dirty(), false);
    }

    #[test]
    fn test_zoom_level() {
        let mut configfile = ConfigFile::new();
        assert_eq!(configfile.is_dirty(), false);

        configfile.set_zoom_level(10.0);
        assert_eq!(configfile.get_zoom_level(), 10.0);
        assert_eq!(configfile.is_dirty(), true);

        configfile.set_zoom_level(0.0);
        assert_eq!(configfile.get_zoom_level(), 0.0);

        configfile.set_zoom_level(21.0);
        assert_eq!(configfile.get_zoom_level(), 21.0);

        configfile.write();
        assert_eq!(configfile.is_dirty(), false);
    }

    #[test]
    fn test_resolution() {
        let mut configfile = ConfigFile::new();
        assert_eq!(configfile.is_dirty(), false);

        configfile.set_resolution(1920, 1080);
        assert_eq!(configfile.get_resolution(), (1920, 1080));
        assert_eq!(configfile.is_dirty(), true);

        configfile.set_resolution(800, 600);
        assert_eq!(configfile.get_resolution(), (800, 600));
        assert_eq!(configfile.is_dirty(), true);

        configfile.write();
        assert_eq!(configfile.is_dirty(), false);
    }

    #[test]
    fn test_fullscreen() {
        let mut configfile = ConfigFile::new();

        assert_eq!(configfile.is_dirty(), false);

        configfile.set_fullscreen(true);
        assert_eq!(configfile.is_fullscreen(), true);
        assert_eq!(configfile.is_dirty(), true);

        configfile.set_fullscreen(false);
        assert_eq!(configfile.is_fullscreen(), false);
        assert_eq!(configfile.is_dirty(), true);

        configfile.write();
        assert_eq!(configfile.is_dirty(), false);
    }
}
