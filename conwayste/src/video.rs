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

use ggez::{Context, graphics, conf::FullscreenType};
use std::num::Wrapping;

#[derive(Debug, Clone, PartialEq, Copy)]
struct Resolution {
    w: f32,
    h: f32
}

/// For now, conwayste supports a `16:9` aspect ratio only.
const DISPLAY_MODES: [Resolution; 5]  = [
    Resolution {w: 1280.0, h: 720.0},
    Resolution {w: 1366.0, h: 768.0},
    Resolution {w: 1600.0, h: 900.0},
    Resolution {w: 1920.0, h: 1080.0},
    Resolution {w: 2560.0, h: 1440.0},
];

/// This manages the supported resolutions.
#[derive(Debug, Clone)]
struct DisplayModeManager {
    index: Wrapping<usize>,
    modes: Vec<Resolution>,
    opt_refresh_rate: Option<i32>,
}

impl DisplayModeManager {
    pub fn new() -> DisplayModeManager {
        DisplayModeManager {
            index: Wrapping(usize::max_value()),
            modes: Vec::new(),
            opt_refresh_rate: None,
        }
    }

    /// Adds a new display mode and removes duplicates.
    pub fn add_mode(&mut self, new_mode: Resolution) {
        for mode in DISPLAY_MODES.iter() {
            if mode == &new_mode {
                self.modes.push(new_mode);
            }
        }
        self.modes.dedup();
    }

    /// Sets the game refresh rate. Right now this does not do anything.
    pub fn set_refresh_rate(&mut self, refresh_rate: i32) {
        self.opt_refresh_rate = Some(refresh_rate);
    }

    /// Prints the supported display modes in debug mode.
    pub fn print_supported_modes(&self) {
        println!("Supported Resolutions Determined:");

        for mode in self.modes.iter() {
            println!("Width: {}, Height: {}", mode.w, mode.h);
        }
    }

    /// Advances to the next resolution.
    pub fn set_next_supported_resolution(&mut self) -> (f32, f32) {
        self.index = (self.index + Wrapping(1usize)) % Wrapping(self.modes.len());
        let display_mode = self.modes.get(self.index.0).unwrap();
        (display_mode.w, display_mode.h)
    }
}

#[derive(Debug, Clone)]
pub struct VideoSettings {
    pub resolution    : (f32, f32),
    pub is_fullscreen :       bool,
    res_manager       : DisplayModeManager,

}

impl VideoSettings {
    pub fn new() -> VideoSettings {
        VideoSettings {
            resolution: (0.0, 0.0),
            is_fullscreen: false,
            res_manager: DisplayModeManager::new(),
        }
    }

/*
 * FIXME
 * as of ggez 0.5.1, there wasn't an obvious way to query the supported display modes.
 * Likely have to dig into wininit to find the answer
    /// We query Wininit (?) to see what resolutions are supported.
    /// This intersected with the `DISPLAY_MODES` list.
    pub fn gather_display_modes(&mut self, ctx: &Context) -> GameResult<()>  {
        let sdl_context =  &ctx.gfx_context;
        let sdl_video = sdl_context.video()?;

        let num_of_display_modes = sdl_video.num_display_modes(0)?;

        for x in  0..num_of_display_modes {
            let display_mode = sdl_video.display_mode(0, x)?;
            let resolution = Resolution {
                w: display_mode.w as f32,
                h: display_mode.h as f32
            };

            self.res_manager.add_mode(resolution);

            if self.res_manager.opt_refresh_rate.is_none() {
                self.res_manager.set_refresh_rate(display_mode.refresh_rate);
            }
        }

        Ok(())
    }
*/

    /// For debug, we have the option to print the supported resolutions.
    pub fn print_resolutions(&self) {
        self.res_manager.print_supported_modes();
    }

    /// Gets the current active resolution.
    pub fn get_active_resolution(&self) -> (f32, f32) {
        self.resolution
    }

    /// Sets the current active resolution and updates the SDL context.
    pub fn set_active_resolution(&mut self, _ctx: &mut Context, w: f32, h: f32) {
        self.resolution = (w,h);
        self.refresh_game_resolution(_ctx, w as i32, h as i32);
    }

    /// Advances to the next supported game resolution, in-order.
    pub fn advance_to_next_resolution(&mut self, ctx: &mut Context) {
        let (width, height) = self.res_manager.set_next_supported_resolution();
        self.set_active_resolution(ctx, width, height);

        info!("{:?}", (width, height));
    }

    /// Updates the SDL video context to the supplied resolution.
    fn refresh_game_resolution(&mut self, ctx: &mut Context, w: i32, h: i32) {
        if w != 0 && h != 0 {
            let _ = graphics::set_drawable_size(ctx, w as f32, h as f32);
        }
    }

/*
 * FIXME
 * as of ggez 0.5.1, there wasn't an obvious way to query whether the window is fullscreen or not.
 * Likely have to dig into wininit to find the answer.
 * */
    /// Toggles fullscreen mode within the SDL video context
    pub fn toggle_fullscreen(&mut self, ctx: &mut Context) {
        let fs_type = if self.is_fullscreen {
            FullscreenType::Windowed
        } else {
            FullscreenType::True
        };
        let _ = graphics::set_fullscreen(ctx, fs_type);
    }
}
