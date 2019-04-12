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

use crate::constants::{CONFIG_FILE_PATH, DEFAULT_ZOOM_LEVEL};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

/// Settings contains all of the user's configurable settings for this game. These *should* be
/// modified within the game, but one can always edit this file directly. The game will fail to
/// load if there are any errors parsing the `conwayste.toml` file.
// Top-level view of config toml file
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct Settings {
    user: UserNetSettings,
    gameplay: GamePlaySettings,
    video: VideoSettings,
    audio: AudioSettings,
}

/// This will decode from the [user] section and contains settings for this user relevant to
/// network (multiplayer) game play.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct UserNetSettings {
    name: String,
}

impl Default for UserNetSettings {
    fn default() -> Self {
        UserNetSettings {
            name: "JohnConway".to_owned(),
        }
    }
}

/// Graphics-related settings like resolution, fullscreen, and more!
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct VideoSettings {
    resolution_x: i32,
    resolution_y: i32,
    fullscreen: bool,
}

/// Audio-related settings like sound and music levels.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AudioSettings {
    master: u8,
    music: u8,
}

impl Default for AudioSettings {
    fn default() -> Self {
        AudioSettings {
            master: 100,
            music: 100,
        }
    }
}

/// Gameplay-related settings. Pretty empty for now.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct GamePlaySettings {
    zoom: f32,
}

impl Default for GamePlaySettings {
    fn default() -> Self {
        GamePlaySettings {
            zoom: DEFAULT_ZOOM_LEVEL,
        }
    }
}

impl Settings {
    /// Writes the default `conwayste.toml` configuration file.
    pub fn write_default_config(&self, path: &str) {
        let toml = toml::to_string(&self).unwrap();
        let mut foptions = OpenOptions::new();
        let mut f = foptions.write(true).create_new(true).open(path).unwrap();
        let _ = f.write(toml.as_bytes());
    }

    /// Writes the in-memory config to the `conwayste.toml` configuration file.
    pub fn write_config(&self) {
        let mut foptions = OpenOptions::new();
        let mut f = foptions.write(true).open(CONFIG_FILE_PATH).unwrap();
        let toml = toml::to_string(&self).unwrap();
        let _ = f.write(toml.as_bytes());
    }

    /// Creates the default configuration with mostly invalid settings.
    pub fn new() -> Self {
        let mut settings: Settings = Default::default();
        // TODO: randomized settings.user.name
        settings
    }

    /// Helper to simply copy the local file copy to memory.
    fn update_settings(&mut self, new_config: Settings) {
        self.user.name = new_config.user.name;
        self.gameplay.zoom = new_config.gameplay.zoom;
        self.video.fullscreen = new_config.video.fullscreen;
        self.video.resolution_x = new_config.video.resolution_x;
        self.video.resolution_y = new_config.video.resolution_y;
        self.audio.master = new_config.audio.master;
        self.audio.music = new_config.audio.music;
    }

    /// Initializes the in-memory configuration settings from an
    /// already existing `conwayste.toml` file or or with the
    /// default settings. This will fail if the toml file cannot
    /// be parsed correctly.
    pub fn initialize(&mut self, path: &str) {
        if Path::exists(Path::new(path)) {
            let mut toml = String::new();
            {
                let mut foptions = OpenOptions::new();
                let mut f = foptions.read(true).open(path).unwrap();
                f.read_to_string(&mut toml).unwrap();
            }

            let toml_str = &toml.as_str();
            let config: Settings = toml::from_str(toml_str).unwrap();

            self.update_settings(config);
        } else {
            self.write_default_config(path);
        };
    }
}

/// Pretty straightforward. If the file is `dirty`, then all of the configuration settings
/// will be flushed down to the local file.
pub struct Config {
    path: Option<String>,
    settings: Settings,
    dirty: bool,
}

impl Config {
    /// Creates a new manager to handle the conwayste configuration settings.
    pub fn new() -> Config {
        let mut config = Settings::new();

        Config {
            path: Some(String::from(CONFIG_FILE_PATH)),
            settings: config,
            dirty: false,
        }
    }

    pub fn set_path(&mut self, path: String) -> &mut Self {
        self.path = Some(path);
        self.set_dirty()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.map(|p| p.as_str())
    }

    /// Prints the configuration via `:?` for sanity checking.
    pub fn print_to_screen(&self) {
        println!("{:#?}\nDirty:{}", self.settings, self.dirty);
    }

    /// Sets the configuration file as dirty.
    fn set_dirty(&mut self) -> &mut Self {
        self.dirty = true;
        self
    }

    /// Marks the configuration file as flushed to the disk.
    fn set_clean(&mut self) -> &mut Self {
        self.dirty = false;
        self
    }

    /// Queries to see if the configuration file is dirty or not.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Writes the configuration file to the local filesystem.
    pub fn write(&mut self) -> &mut Self {
        println!("AARON: config write"); //XXX
        self.settings.write_config();
        self.set_clean()
    }

    /// Gets the configuration files copy of what the resolution is as a tuple.
    pub fn resolution(&self) -> (i32, i32) {
        (
            self.settings.video.resolution_x,
            self.settings.video.resolution_y,
        )
    }

    /// Updates the resolution within the configuration file.
    pub fn set_resolution(&mut self, width: i32, height: i32) -> &mut Self {
        self.settings.video.resolution_x = width;
        self.settings.video.resolution_y = height;
        self.set_dirty()
    }

    /// Checks to see if the game was listed as fullscreen within the toml settings.
    pub fn is_fullscreen(&self) -> bool {
        self.settings.video.fullscreen == true
    }

    /// Sets the fullscreen setting with the providied boolean.
    pub fn set_fullscreen(&mut self, is_fullscreen: bool) -> &mut Self {
        self.settings.video.fullscreen = is_fullscreen;
        self.set_dirty()
    }

    /*
     *
    /// Sets the master sound level to the specified value.
    /// Value range is 0 to 100.
    pub fn set_master_sound_level(&mut self, level: u8) -> &mut Self {
        self.settings.audio.master = level;
        self.set_dirty()
    }

    /// Sets the music level to the specified value.
    /// Value range is 0 to 100.
    pub fn set_music_level(&mut self, level: u8) -> &mut Self {
        self.settings.audio.music = level;
        self.set_dirty()
    }

     *
     * TODO once we have audio implemented
     *

    /// Gets the master sound level.
    pub fn master_sound_level(&self) -> u8 {
        self.settings.audio.master
    }

    /// Sets the master sound level to the specified value.
    pub fn music_level(&self) -> u8 {
        self.settings.audio.music
    }
    */

    /// Sets the zoom level within the configuration file.
    pub fn set_zoom_level(&mut self, level: f32) -> &mut Self {
        self.settings.gameplay.zoom = level;
        self.set_dirty()
    }

    /// Gets the zoom level file specified in the toml file.
    pub fn zoom_level(&self) -> f32 {
        self.settings.gameplay.zoom
    }

    /*
        /// Gets the player name specified within the configuration file.
        pub fn player_name(&self) -> &str {
            &self.settings.user.name
        }

        /// Sets the player name within the configuration file.
        pub fn set_player_name(&mut self, name: String) -> &mut Self {
            self.settings.user.name = name;
            self.set_dirty()
        }
    */
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_init_default_settings() {
        let settings = Settings::new();

        assert_eq!(settings.audio.master, 100);
        assert_eq!(settings.audio.music, 100);
        assert_eq!(settings.video.fullscreen, false);
        assert_eq!(settings.video.resolution_x, 0);
        assert_eq!(settings.video.resolution_y, 0);
        assert_eq!(settings.gameplay.zoom, DEFAULT_ZOOM_LEVEL);
        assert_eq!(settings.user.name, "JohnConway");
    }

    #[test]
    fn test_update_settings() {
        let mut settings = Settings::new();
        let mut secondary_settings = Settings::new();

        secondary_settings.user.name = String::from("TestUser");
        secondary_settings.audio.master = 50;
        secondary_settings.audio.music = 50;
        secondary_settings.video.fullscreen = true;
        secondary_settings.gameplay.zoom = 1000.0;

        settings.update_settings(secondary_settings);
        assert_eq!(settings.audio.master, 50);
        assert_eq!(settings.audio.music, 50);
        assert_eq!(settings.video.fullscreen, true);
        assert_eq!(settings.video.resolution_x, 0);
        assert_eq!(settings.video.resolution_y, 0);
        assert_eq!(settings.gameplay.zoom, 1000.0);
        assert_eq!(settings.user.name, "TestUser");
    }

    #[test]
    fn test_config_cleanliness() {
        let mut config = Config::new();

        assert_eq!(config.is_dirty(), false);

        config.set_dirty();
        assert_eq!(config.is_dirty(), true);

        config.write();
        assert_eq!(config.is_dirty(), false);
    }

    #[test]
    fn test_modify_default_config_and_write() {
        let mut config = Config::new();

        assert_eq!(config.is_dirty(), false);

        config.set_zoom_level(10.0);
        assert_eq!(config.get_zoom_level(), 11.0);
        assert_eq!(config.is_dirty(), true);

        config.write();
        assert_eq!(config.is_dirty(), false);
    }

    #[test]
    fn test_zoom_level() {
        let mut config = Config::new();
        assert_eq!(config.is_dirty(), false);

        config.set_zoom_level(10.0);
        assert_eq!(config.get_zoom_level(), 10.0);
        assert_eq!(config.is_dirty(), true);

        config.set_zoom_level(0.0);
        assert_eq!(config.get_zoom_level(), 0.0);

        config.set_zoom_level(21.0);
        assert_eq!(config.get_zoom_level(), 21.0);

        config.write();
        assert_eq!(config.is_dirty(), false);
    }

    #[test]
    fn test_resolution() {
        let mut config = Config::new();
        assert_eq!(config.is_dirty(), false);

        config.set_resolution(1920, 1080);
        assert_eq!(config.get_resolution(), (1920, 1080));
        assert_eq!(config.is_dirty(), true);

        config.set_resolution(800, 600);
        assert_eq!(config.get_resolution(), (800, 600));
        assert_eq!(config.is_dirty(), true);

        config.write();
        assert_eq!(config.is_dirty(), false);
    }

    #[test]
    fn test_fullscreen() {
        let mut config = Config::new();

        assert_eq!(config.is_dirty(), false);

        config.set_fullscreen(true);
        assert_eq!(config.is_fullscreen(), true);
        assert_eq!(config.is_dirty(), true);

        config.set_fullscreen(false);
        assert_eq!(config.is_fullscreen(), false);
        assert_eq!(config.is_dirty(), true);

        config.write();
        assert_eq!(config.is_dirty(), false);
    }
}
