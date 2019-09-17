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

use ggez::{Context, graphics, GameResult, conf::FullscreenType};

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub struct Resolution {
    pub w: f32,
    pub h: f32,
}

impl From<(f32, f32)> for Resolution {
    fn from(src: (f32, f32)) -> Resolution {
        Resolution{ w: src.0, h: src.1 }
    }
}

/*
const DISPLAY_MODES: [Resolution; 5]  = [
    Resolution {w: 1280, h: 720},
    Resolution {w: 1366, h: 768},
    Resolution {w: 1600, h: 900},
    Resolution {w: 1920, h: 1080},
    Resolution {w: 2560, h: 1440},
];
*/

#[derive(Debug, Clone)]
pub struct VideoSettings {
    resolution:        Resolution,
    pub is_fullscreen: bool,
}

impl VideoSettings {
    pub fn new() -> VideoSettings {
        VideoSettings {
            resolution: Resolution::default(),
            is_fullscreen: false,
        }
    }

    /// Gets the current active resolution.
    pub fn get_resolution(&self) -> Resolution {
        self.resolution
    }

    /// Sets the `resolution` field.
    /// If `refresh` is true, calls `update_resolution` to actually resize the window.
    pub fn set_resolution(&mut self, ctx: &mut Context, res: Resolution, refresh: bool) -> GameResult<()> {
        self.resolution = res;
        if refresh {
            self.update_resolution(ctx, res.w, res.h)?;
        }
        Ok(())
    }

    /// Resizes the window based on the `resolution` field.
    fn update_resolution(&mut self, ctx: &mut Context, w: f32, h: f32) -> GameResult<()> {
        graphics::set_drawable_size(ctx, w, h)?;
        Ok(())
    }

    /// Makes us fullscreen or not based on the `is_fullscreen` field.
    pub fn update_fullscreen(&mut self, ctx: &mut Context) -> GameResult<()> {
        let fs_type = if self.is_fullscreen {
            FullscreenType::Desktop
        } else {
            FullscreenType::Windowed
        };
        graphics::set_fullscreen(ctx, fs_type)?;
        Ok(())
    }
}
