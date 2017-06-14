extern crate ggez;


#[derive(Debug, Clone)]
// TODO this should be moved into a video/gfx module
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

    pub fn getResolution(&self) -> &(u16, u16) {
        &self.resolution
    }

    pub fn setResolution(&mut self, x: u16, y: u16) {
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
