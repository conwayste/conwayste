extern crate ggez;

use ggez::Context;
use sdl2::video::FullscreenType;

#[derive(Debug, Clone)]
pub enum ScreenResolution {
    PX800X600,
    PX1024X768,
    PX1200X960,
    PX1920X1080,
}

#[derive(Debug, Clone)]
pub struct VideoSettings {
    pub resolution : (u16, u16),
}

impl VideoSettings {
    pub fn new() -> VideoSettings {
        VideoSettings {
            resolution: (0,0),
        }
    }

    pub fn get_resolution(&self) -> &(u16, u16) {
        &self.resolution
    }

    pub fn set_resolution(&mut self, x: u16, y: u16) {
        self.resolution = (x,y)
    }
}

pub fn get_resolution_str(x: ScreenResolution) -> &'static str {
    match x {
        ScreenResolution::PX800X600 => {
            "800 x 600"
        }
        ScreenResolution::PX1024X768 => {
            "1024 x 768"
        }
        ScreenResolution::PX1200X960 => {
            "1200 x 960"
        }
        ScreenResolution::PX1920X1080 => {
            "1920 x 1080"
        }
    }
}

pub fn toggle_full_screen(_ctx: &mut Context) -> bool {
    let renderer = &mut _ctx.renderer;
    let window = renderer.window_mut().unwrap();
    let wlflags = window.window_flags();
    let fullscreen_type = FullscreenType::from_window_flags(wlflags);
    let mut new_fs_type = FullscreenType::Off;

    match fullscreen_type {
        FullscreenType::Off => {new_fs_type = FullscreenType::True;}
        FullscreenType::True => {new_fs_type = FullscreenType::Off;}
        _ => {}
    }
    let _ = window.set_fullscreen(new_fs_type);
    
    new_fs_type == FullscreenType::True
}
