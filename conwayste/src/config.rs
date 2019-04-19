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

use crate::constants::{CONFIG_FILE_PATH, DEFAULT_ZOOM_LEVEL, MIN_CONFIG_FLUSH_TIME};
use std::error::Error;
use std::time::Instant;

#[cfg(not(test))]
use std::fs::OpenOptions;
#[cfg(not(test))]
use std::io::{Read, Write};
#[cfg(not(test))]
use std::path::Path;

type TomlMap = toml::map::Map<String, toml::Value>;
use toml::Value;

/// Settings contains all of the user's configurable settings for this game. These *should* be
/// modified within the game, but one can always edit this file directly. The game will fail to
/// load if there are any errors parsing the `conwayste.toml` file.
// Top-level view of config toml file
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    pub user: UserNetSettings,
    pub gameplay: GamePlaySettings,
    pub video: VideoSettings,
    pub audio: AudioSettings,
}

/// This will decode from the [user] section and contains settings for this user relevant to
/// network (multiplayer) game play.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserNetSettings {
    pub name: String,
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
pub struct VideoSettings {
    pub resolution_x: i32,
    pub resolution_y: i32,
    pub fullscreen: bool,
}

/// Audio-related settings like sound and music levels.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioSettings {
    pub master: u8,
    pub music: u8,
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
pub struct GamePlaySettings {
    pub zoom: f32,
}

impl Default for GamePlaySettings {
    fn default() -> Self {
        GamePlaySettings {
            zoom: DEFAULT_ZOOM_LEVEL,
        }
    }
}

impl Settings {
    /// Creates the default configuration with default settings.
    pub fn new() -> Self {
        let settings: Settings = Default::default();
        // TODO: randomized settings.user.name
        settings
    }
}

/// Config manages how Settings are loaded and stored to the filesystem.
pub struct Config {
    settings: Settings, // the actual settings
    path: String,
    // TODO: following two items in a RefCell
    dirty: bool, // config needs to be flushed?
    flush_time: Option<Instant>,
    #[cfg(test)]
    pub dummy_file_data: Option<String>,
}

impl Config {
    /// Creates a Config with default settings.
    pub fn new() -> Config {
        let config = Settings::new();

        Config {
            settings: config,
            path: String::from(CONFIG_FILE_PATH),
            dirty: false,
            flush_time: None,
            #[cfg(test)]
            dummy_file_data: None,
        }
    }

    pub fn set_path(&mut self, path: String) -> &mut Self {
        self.path = path;
        self.set_dirty()
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
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

    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        let mut toml_str = String::new();
        #[cfg(not(test))]
        {
            let mut foptions = OpenOptions::new();
            let mut f = foptions.read(true).open(&self.path)?;
            f.read_to_string(&mut toml_str)?;
        }

        #[cfg(test)]
        {
            toml_str = self.dummy_file_data.as_ref().unwrap().clone();
        }

        let default_settings: Settings = Default::default();
        let default_string: String = toml::to_string(&self.settings)?;
        let mut result_map: TomlMap = toml::from_str(default_string.as_str())?; // set the result to default
        println!("BEFORE: result_map is {:#?}", result_map);
        let map_from_file: TomlMap = toml::from_str(toml_str.as_str())?;
        println!("map_from_file is {:?}", map_from_file);
        for (ref section_name, ref table_val) in map_from_file.iter() {
            println!("section_name is {:?} and table_val is {:?}", section_name, table_val);
            match table_val {
                Value::Table(table) => {
                    for (ref field, ref value) in table.iter() {
                        println!("field is {:?} and value is {:?}", field, value);
                        let table_ref: &mut Value = result_map.get_mut(*section_name).unwrap();
                        match table_ref {
                            Value::Table(ref mut result_table) => {
                                println!("yay we did it");
                                let value_ref: &mut Value = result_table.get_mut(*field).unwrap();
                                *value_ref = (*value).clone();
                            }
                            _ => panic!("expected a i dunno")
                        }

                    }
                }
                _ => panic!("expected a table")
            }
        }
        println!("AFTER: result_map is {:#?}", result_map);
        let mut result_string = toml::to_string(&result_map)?;
        self.settings = toml::from_str(result_string.as_str())?;
        println!("self.settings.video.fullscreen is {:?}", self.settings.video.fullscreen);
        Ok(())
    }

    /// Check if file at `self.path` exists. If it exists, settings are read from that path.
    /// Otherwise, the current settings are written to that path. Note: `Config::new()` returns
    /// a `Config` with default settings.
    pub fn load_or_create_default(&mut self) -> Result<(), Box<dyn Error>> {
        let path_exists;
        #[cfg(not(test))]
        {
            path_exists = Path::exists(Path::new(&self.path));
        }

        #[cfg(test)]
        {
            path_exists = self.dummy_file_data.is_some();
        }

        if path_exists {
            self.load()?;
        } else {
            self.force_flush()?;
        };
        Ok(())
    }

    /// Save to file unconditionally.
    pub fn force_flush(&mut self) -> Result<(), Box<dyn Error>> {
        let toml_str = toml::to_string(&self.settings)?;

        #[cfg(not(test))]
        {
            let mut foptions = OpenOptions::new();
            let mut f = foptions.write(true).create_new(true).open(&self.path)?;
            f.write(toml_str.as_bytes())?;
        }

        #[cfg(test)]
        {
            self.dummy_file_data = Some(toml_str);
        }

        self.set_clean();
        self.flush_time = Some(Instant::now());

        Ok(())
    }

    /// Flush the config to disk if dirty and sufficient time has passed (`MIN_CONFIG_FLUSH_TIME`)
    /// since the previous flush. It is recommended to call this frequently -- typically the cost
    /// is low.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if flushed.
    /// * `Ok(false)` if not flushed because not dirty or because not enough
    /// time has passed.
    /// * `Err(...)` if a flush was attempted but there was an error.
    pub fn flush(&mut self) -> Result<bool, Box<dyn Error>> {
        if self.is_dirty()
            && (self.flush_time.is_none()
                || Instant::now() - self.flush_time.unwrap() > MIN_CONFIG_FLUSH_TIME)
        {
            self.force_flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get(&self) -> &Settings {
        &self.settings
    }

    /// Accepts a closure taking a mutable reference to `Settings`. Within the closure, it can be
    /// modified. When the closure returns, the config will be marked as dirty.
    ///
    /// ```rust,ignore
    /// config.modify(|settings| {
    ///     settings.video.fullscreen = true;
    /// });
    /// ```
    pub fn modify<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Settings),
    {
        f(&mut self.settings);
        self.set_dirty();
        // TODO: pass a clone of the settings above, and then validate afterwards. If validation
        // passes, then save the clone.
    }

    /////////// Convenience Methods ///////////
    pub fn set_resolution(&mut self, w: i32, h: i32) {
        self.modify(|settings| {
            settings.video.resolution_x = w;
            settings.video.resolution_y = h;
        });
    }
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
    fn test_config_cleanliness() {
        let mut config = Config::new();

        assert_eq!(config.is_dirty(), false);

        config.set_dirty();
        assert_eq!(config.is_dirty(), true);

        config.force_flush().unwrap();
        assert_eq!(config.is_dirty(), false);
    }

    #[test]
    fn test_modify_default_config_and_write() {
        let mut config = Config::new();

        assert_eq!(config.is_dirty(), false);

        config.modify(|settings| {
            settings.gameplay.zoom = 10.0;
        });
        assert_eq!(config.get().gameplay.zoom, 10.0);
        assert_eq!(config.is_dirty(), true);

        config.force_flush().unwrap();
        assert_eq!(config.is_dirty(), false);
    }
}
