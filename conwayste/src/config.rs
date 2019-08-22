/*  Copyright 2017-2019 the Conwayste Developers.
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
use std::fmt;
use std::time::Instant;

use std::fs::OpenOptions;
use std::io::Read;
#[cfg(not(test))]
use std::io::Write;
#[cfg(not(test))]
use std::path::Path;

type TomlMap = toml::map::Map<String, toml::Value>;
use toml::Value;

#[derive(Debug)]
pub struct ConfigError {
    pub msg: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}

impl Error for ConfigError {}

fn new_config_error(msg: String) -> Box<dyn Error> {
    Box::new(ConfigError { msg })
}

lazy_static! {
    /// The default configuration, in TOML format.
    static ref DEFAULT_STRING: String = {
        let default_settings: Settings = Default::default();
        toml::to_string(&default_settings).unwrap()
    };

    /// A TomlMap for the `DEFAULT_STRING`.
    static ref DEFAULT_MAP: TomlMap = toml::from_str(DEFAULT_STRING.as_str()).unwrap();

    /// Same as `DEFAULT_STRING` but as a TOML comment and with version string at top.
    static ref COMMENTED_DEFAULT_STRING: String = {
        let mut s = String::new();
        s.push_str(&format!("############ Default config for Conwayste v{} ##########\n", version!()));
        for default_line in DEFAULT_STRING.split("\n") {
            s.push_str(&format!("# {}\n", default_line));
        }
        s
    };
}

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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoSettings {
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub fullscreen: bool,
}

impl Default for VideoSettings {
    fn default() -> Self {
        VideoSettings {
            resolution_x: 1024,
            resolution_y: 768,
            fullscreen: false,
        }
    }
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
    settings: Settings,          // The actual settings
    path: String,                // Path to config file. `conwayste.toml` by default.
    dirty: bool,                 // Config needs to be flushed to disk?
    flush_time: Option<Instant>, // Last time (if any) that we flushed to disk.
    #[cfg(test)]
    pub dummy_file_data: Option<String>, // for mocking file reads and writes
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

    #[allow(dead_code)]
    pub fn set_path(&mut self, path: String) -> &mut Self {
        self.path = path;
        self.set_dirty()
    }

    #[allow(dead_code)]
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
        #[allow(unused_assignments)]
        let mut toml_str = String::new();
        #[cfg(test)]
        {
            toml_str = self.dummy_file_data.as_ref().unwrap().clone();
        }
        if !cfg!(test) {
            let mut foptions = OpenOptions::new();
            let mut f = foptions.read(true).open(&self.path)?;
            f.read_to_string(&mut toml_str)?;
        }

        let mut result_map: TomlMap = DEFAULT_MAP.clone();
        let map_from_file: TomlMap = toml::from_str(toml_str.as_str())?;
        for (section_name, ref table_val) in map_from_file.iter() {
            match table_val {
                Value::Table(table) => {
                    for (field, ref value) in table.iter() {
                        let table_ref: &mut Value =
                            result_map.get_mut(section_name).ok_or_else(|| {
                                new_config_error(format!("unexpected section: {}", section_name))
                            })?;
                        match table_ref {
                            Value::Table(ref mut result_table) => {
                                let value_ref: &mut Value =
                                    result_table.get_mut(field).ok_or_else(|| {
                                        new_config_error(format!(
                                            "in section {}: unexpected field: {}",
                                            section_name, field
                                        ))
                                    })?;

                                let (expected_type, actual_type) =
                                    (value_ref.type_str(), value.type_str());
                                if expected_type != actual_type {
                                    let msg = format!("in section {}: unexpected data type for field: {}; expected {} but actually {}",
                                                      section_name, field, expected_type, actual_type);
                                    return Err(new_config_error(msg));
                                }
                                *value_ref = (*value).clone();
                            }
                            _ => unimplemented!(
                                "We have a top-level field in our config but encountered a section"
                            ), // we don't have any yet
                        }
                    }
                }
                _ => {
                    let msg = format!("unexpected top-level field: {}", section_name);
                    return Err(new_config_error(msg));
                }
            }
        }
        let result_string = toml::to_string(&result_map)?;
        self.settings = toml::from_str(result_string.as_str())?;
        Ok(())
    }

    /// Check if file at `self.path` exists. If it exists, settings are read from that path.
    /// Otherwise, the current settings are written to that path. Note: `Config::new()` returns
    /// a `Config` with default settings.
    pub fn load_or_create_default(&mut self) -> Result<(), Box<dyn Error>> {
        let path_exists;
        #[cfg(test)]
        {
            path_exists = self.dummy_file_data.is_some();
        }
        #[cfg(not(test))]
        {
            path_exists = Path::exists(Path::new(&self.path));
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
        let full_toml_str = toml::to_string(&self.settings)?;
        let settings_map: TomlMap = toml::from_str(full_toml_str.as_str())?;
        let mut result_map = TomlMap::new();
        // compare each thing in DEFAULT_MAP vs settings_map; if different, add the latter to
        // result_map
        for (section_name, default_table_val) in DEFAULT_MAP.iter() {
            let default_table = default_table_val.as_table().unwrap();
            let settings_table_val = settings_map.get(section_name).unwrap();
            let settings_table = settings_table_val.as_table().unwrap();
            for (field_name, default_val) in default_table.iter() {
                let settings_val = settings_table.get(field_name).unwrap();
                assert_eq!(
                    default_val.type_str(),
                    settings_val.type_str(),
                    "types do not match"
                );
                if default_val != settings_val {
                    if !result_map.contains_key(section_name) {
                        result_map.insert(section_name.clone(), Value::Table(TomlMap::new()));
                    }
                    let result_table = result_map
                        .get_mut(section_name)
                        .unwrap()
                        .as_table_mut()
                        .unwrap();

                    // put in result_map
                    result_table.insert(field_name.clone(), settings_val.clone());
                }
            }
        }
        let mut toml_str = toml::to_string(&result_map)?;
        toml_str.push_str("\n");
        toml_str.push_str(&COMMENTED_DEFAULT_STRING);

        #[cfg(test)]
        {
            self.dummy_file_data = Some(toml_str);
        }

        #[cfg(not(test))]
        {
            let mut foptions = OpenOptions::new();
            let mut f = foptions.write(true).create(true).open(&self.path)?;
            f.set_len(0)?;
            f.write(toml_str.as_bytes())?;
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

    #[allow(dead_code)]
    pub fn flush_time(&self) -> Option<Instant> {
        self.flush_time
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
    pub fn get_resolution(&self) -> (u32, u32) {
        (self.settings.video.resolution_x, self.settings.video.resolution_y)
    }

    pub fn set_resolution(&mut self, w: u32, h: u32) {
        self.modify(|settings| {
            settings.video.resolution_x = w;
            settings.video.resolution_y = h;
        });
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use std::ops::{AddAssign, SubAssign};
    use std::time::Duration;

    fn adjust_flush_time(settings: &mut Config, adjustment: Duration, sign: isize) {
        if sign == 0 {
            panic!("unexpected sign value");
        }
        if sign > 0 {
            settings.flush_time.as_mut().unwrap().add_assign(adjustment);
        } else {
            settings.flush_time.as_mut().unwrap().sub_assign(adjustment);
        }
    }
    #[test]
    fn test_init_default_settings() {
        let settings = Settings::new();

        assert_eq!(settings.audio.master, 100);
        assert_eq!(settings.audio.music, 100);
        assert_eq!(settings.video.fullscreen, false);
        //assert_eq!(settings.video.resolution_x, 1024);
        //assert_eq!(settings.video.resolution_y, 768);
        assert_eq!(settings.gameplay.zoom, DEFAULT_ZOOM_LEVEL);
        //assert_eq!(settings.user.name, "JohnConway");
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

    #[test]
    fn test_load_or_create_default_new_file() {
        let mut config = Config::new();
        config.load_or_create_default().unwrap();
        let filedata = config.dummy_file_data.unwrap(); // this is the default config
        let mut filedata_lines = filedata.as_str().split("\n");
        // Just verify initial line and '#' at start of each line

        // Since this is the default config, there are no (un-commented) config lines.
        assert_eq!(filedata_lines.next(), Some(""));
        let mut blank_lines = 0;
        for line in filedata_lines {
            // a line should be either blank or be a comment
            let opt_first_char = line.chars().next();
            if opt_first_char.is_none() {
                blank_lines += 1;
                continue;
            }
            assert_eq!(opt_first_char, Some('#'));
        }
        assert_eq!(blank_lines, 1);
    }

    #[test]
    fn test_load_or_create_default_existing_valid_file() {
        let mut config = Config::new();
        let existing_filedata = "[video]\nfullscreen = true\n[audio]\nmaster = 69\n".to_owned();
        config.dummy_file_data = Some(existing_filedata.clone());
        config.load_or_create_default().unwrap();
        let new_filedata = config.dummy_file_data.take().unwrap();
        assert_eq!(existing_filedata, new_filedata); // since file was already there, should not be changed

        // verify that config was updated
        assert_eq!(config.get().video.fullscreen, true);
        assert_eq!(config.get().audio.master, 69);
    }

    #[test]
    fn test_load_or_create_default_invalid_section_name() {
        let mut config = Config::new();
        let existing_filedata = "[invalid]\nfullscreen = true\n".to_owned();
        config.dummy_file_data = Some(existing_filedata.clone());

        let box_err = config.load_or_create_default().unwrap_err();
        let err = box_err.downcast_ref::<ConfigError>().unwrap();
        assert_eq!(err.msg.as_str(), "unexpected section: invalid");

        let new_filedata = config.dummy_file_data.take().unwrap();
        assert_eq!(existing_filedata, new_filedata); // since file was already there, should not be changed
    }

    #[test]
    fn test_load_or_create_default_invalid_field_name() {
        let mut config = Config::new();
        let existing_filedata = "[video]\ninvalid = true\n".to_owned();
        config.dummy_file_data = Some(existing_filedata.clone());

        let box_err = config.load_or_create_default().unwrap_err();
        let err = box_err.downcast_ref::<ConfigError>().unwrap();
        assert_eq!(
            err.msg.as_str(),
            "in section video: unexpected field: invalid"
        );

        let new_filedata = config.dummy_file_data.take().unwrap();
        assert_eq!(existing_filedata, new_filedata); // since file was already there, should not be changed
    }

    #[test]
    fn test_load_or_create_default_invalid_field_type() {
        let mut config = Config::new();
        let existing_filedata = "[video]\nfullscreen = 3\n".to_owned();
        config.dummy_file_data = Some(existing_filedata.clone());

        let box_err = config.load_or_create_default().unwrap_err();
        let err = box_err.downcast_ref::<ConfigError>().unwrap();
        assert_eq!(err.msg.as_str(), "in section video: unexpected data type for field: fullscreen; expected boolean but actually integer");

        let new_filedata = config.dummy_file_data.take().unwrap();
        assert_eq!(existing_filedata, new_filedata); // since file was already there, should not be changed
    }

    #[test]
    fn test_load_or_create_default_invalid_top_level_field() {
        let mut config = Config::new();
        let existing_filedata = "fullscreen = true\n".to_owned();
        config.dummy_file_data = Some(existing_filedata.clone());

        let box_err = config.load_or_create_default().unwrap_err();
        let err = box_err.downcast_ref::<ConfigError>().unwrap();
        assert_eq!(err.msg.as_str(), "unexpected top-level field: fullscreen");

        let new_filedata = config.dummy_file_data.take().unwrap();
        assert_eq!(existing_filedata, new_filedata); // since file was already there, should not be changed
    }

    #[test]
    fn test_flush_should_not_happen_with_fresh_config() {
        let mut config = Config::new();
        assert_eq!(config.flush().unwrap(), false);
    }

    #[test]
    fn test_flush_should_happen_after_change() {
        let mut config = Config::new();
        config.modify(|settings: &mut Settings| {
            settings.video.fullscreen = true;
        });
        assert_eq!(config.flush().unwrap(), true);
    }

    #[test]
    fn test_flush_second_time_immediately_should_not_happen() {
        let mut config = Config::new();
        config.modify(|settings: &mut Settings| {
            settings.video.fullscreen = true;
        });
        assert_eq!(config.flush().unwrap(), true);
        config.modify(|settings: &mut Settings| {
            settings.video.resolution_x = 123;
        });
        assert_eq!(config.flush().unwrap(), false);
    }

    #[test]
    fn test_flush_eventually_happens() {
        let mut config = Config::new();
        config.modify(|settings: &mut Settings| {
            settings.video.fullscreen = true;
        });
        assert_eq!(config.flush().unwrap(), true);
        config.modify(|settings: &mut Settings| {
            settings.video.resolution_x = 123;
        });
        assert_eq!(config.is_dirty(), true);
        adjust_flush_time(&mut config,
            Duration::from_millis(MIN_CONFIG_FLUSH_TIME.as_millis() as u64 + 1),
            -1,
        );

        assert_eq!(config.flush().unwrap(), true);
    }

    #[test]
    fn test_force_flush_should_show_only_changed_value() {
        let mut config = Config::new();
        // this assumes the default for fullscreen is false, which is unlikely to change
        config.modify(|settings: &mut Settings| {
            settings.video.fullscreen = true;
        });
        assert!(config.force_flush().is_ok());
        let filedata = config.dummy_file_data.take().unwrap();
        let filedata_lines: Vec<&str> = filedata.as_str().split("\n").collect();
        assert_eq!(&filedata_lines[0..2], &["[video]", "fullscreen = true",]);

        // also test commented lines after this
        let commented_default_lines: Vec<&str> = COMMENTED_DEFAULT_STRING.split("\n").collect();
        assert_eq!(&filedata_lines[3..], &commented_default_lines[..]);
    }
}
