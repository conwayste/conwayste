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

use ggez::{Context, graphics};
use sdl2::video::{FullscreenType, DisplayMode};
use std::num::Wrapping;

#[derive(Debug, Clone)]
struct DisplayModeManager {
    index: Wrapping<usize>,
    modes: Vec<DisplayMode>,
}

impl DisplayModeManager {
    pub fn new() -> DisplayModeManager {
        DisplayModeManager {
            index: Wrapping(usize::max_value()),
            modes: Vec::new(),
        }
    }

    pub fn add_mode(&mut self, new_mode: DisplayMode) {
            self.modes.push(new_mode);
    }

    pub fn print_supported_modes(&self) {
        info!("Supported Resolutions Discovered:");

        for mode in self.modes.iter() {
            info!("Width: {}, Height: {}, Format: {:?}", mode.w, mode.h, mode.format);
        }
    }

    pub fn set_next_supported_resolution(&mut self) -> (i32, i32) {
        self.index = (self.index + Wrapping(1usize)) % Wrapping(self.modes.len());
        let display_mode = self.modes.get(self.index.0).unwrap();
        (display_mode.w, display_mode.h)
    }
}

#[derive(Debug, Clone)]
pub struct VideoSettings {
    pub resolution    : (i32, i32),
    pub is_fullscreen :       bool,
    res_manager       : DisplayModeManager,

}

impl VideoSettings {
    pub fn new() -> VideoSettings {
        VideoSettings {
            resolution: (0,0),
            is_fullscreen: false,
            res_manager: DisplayModeManager::new(),
        }
    }

    pub fn gather_display_modes(&mut self, _ctx: &Context) {
        let sdl_context =  &_ctx.sdl_context;
        let sdl_video = sdl_context.video().unwrap();

        let num_of_display_modes = sdl_video.num_display_modes(0).unwrap();

        for x in  0..num_of_display_modes {
            let display_mode = sdl_video.display_mode(0, x).unwrap();
            self.res_manager.add_mode(display_mode);
        }
    }

    pub fn print_resolutions(&self) {
        self.res_manager.print_supported_modes();
    }

    pub fn get_active_resolution(&self) -> (i32, i32) {
        self.resolution
    }

    pub fn set_active_resolution(&mut self, _ctx: &mut Context, w: i32, h: i32) {
        self.resolution = (w,h);
        refresh_game_resolution(_ctx, w, h);
    }

    pub fn advance_to_next_resolution(&mut self, _ctx: &mut Context) {
        let (width, height) = self.res_manager.set_next_supported_resolution();
        self.set_active_resolution(_ctx, width, height);

        info!("{:?}", (width, height));
    }

}

pub fn toggle_full_screen(_ctx: &mut Context) -> bool {
    let is_fullscreen;
    if graphics::is_fullscreen(_ctx) {
        is_fullscreen = false;
        graphics::set_fullscreen(_ctx, is_fullscreen);
    }
    else
    {
        is_fullscreen = true;
        graphics::set_fullscreen(_ctx, is_fullscreen);
    }
    is_fullscreen
}

/*
pub fn set_fullscreen(_ctx: &mut Context, fullscreen: bool) -> bool {
    let renderer = &mut _ctx.renderer;
    let window = renderer.window_mut().unwrap();
    let new_fs_type;

    match fullscreen {
        true => {new_fs_type = FullscreenType::True;}
        false => {new_fs_type = FullscreenType::Off;}
    }
    let _ = window.set_fullscreen(new_fs_type);
    
    new_fs_type == FullscreenType::True
}
*/
/*
pub fn get_display_mode(_ctx: &mut Context) -> bool {
    let renderer = &mut _ctx.renderer;
    let window = renderer.window_mut().unwrap();

    match window.display_mode() {
        Ok(x) => {
            println!("Format: {}, W: {}, H: {}", x.format, x.w, x.h);
        }
        Err(x) => { println!("There was nothing to be found for the VSS: {}", x) }
    }
    true
}

pub fn get_current_display_mode(_ctx: &mut Context) -> bool {
    let sdl_context = &mut _ctx.sdl_context;
    let video_subsystem = sdl_context.video().unwrap();
   // let video_subsystem = window.subsystem();

    match video_subsystem.current_display_mode(0) {
        Ok(x) => {
            println!("Format: {}, W: {}, H: {}", x.format, x.w, x.h);
        }
        Err(x) => { println!("There was nothing to be found for the VSS: {}", x) }
    }
    true
}
*/

fn refresh_game_resolution(_ctx: &mut Context, w: i32, h: i32) {
    if w != 0 && h != 0 {
        graphics::set_resolution(_ctx, w as u32, h as u32);
/*
        let ref mut renderer = _ctx.renderer;
        let _ = renderer.set_logical_size(w as u32, h as u32);
        {
            let window = renderer.window_mut().unwrap();
            let _ = window.set_size(w as u32, h as u32);
        }
*/
    }
}

