extern crate conway;
extern crate ggez;
#[macro_use]
extern crate version;

use ggez::conf;
use ggez::event::*;
use ggez::game::{Game, GameState};
use ggez::{GameResult, Context};
use ggez::graphics;
use ggez::graphics::{Rect, Point, Color};
use ggez::timer;
use std::time::Duration;
use std::fs::File;
use conway::{Universe, CellState};
use std::collections::BTreeMap;


const FPS: u32 = 30;
const INTRO_DURATION: f64 = 2.0;
const SCREEN_WIDTH: u32 = 2000;
const SCREEN_HEIGHT: u32 = 1200;


#[derive(PartialEq)]
enum Stage {
    Intro(f64),   // seconds
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
}

// All game state
struct MainState {
    intro_text:          graphics::Text,
    stage:               Stage,
    uni:                 Universe,
    first_gen_was_drawn: bool,
    grid_view:           GridView,
    color_settings:      ColorSettings,
}

struct ColorSettings {
    cell_colors: BTreeMap<CellState, Color>,
    background: Color,
}

impl ColorSettings {
    fn get_color(&self, cell_or_none: Option<CellState>) -> Color {
        match cell_or_none {
            Some(cell) => self.cell_colors[&cell],
            None       => self.background
        }
    }
}


// Controls the mapping between window and game coordinates
struct GridView {
    rect:        Rect,  // the area the game grid takes up on screen
    cell_size:   i32,   // zoom level in window coordinates
    columns:     usize, // width in game coords (should match bitmap/universe width)
    rows:        usize, // height in game coords (should match bitmap/universe height)
    grid_origin: Point, // top-left corner of grid in window coords. (may be outside rect)
    wrap:        bool,  // does this view wrap? (true for universes, false for draw pads)
}

// Then we implement the `ggez::game::GameState` trait on it, which
// requires callbacks for creating the game state, updating it each
// frame, and drawing it.
//
// The `GameState` trait also contains callbacks for event handling
// that you can override if you wish, but the defaults are fine.
impl GameState for MainState {
    fn load(ctx: &mut Context, _conf: &conf::Conf) -> GameResult<MainState> {
        let font = graphics::Font::new(ctx, "DejaVuSerif.ttf", 48).unwrap();
        let intro_text = graphics::Text::new(ctx, "WAYSTE EM!", &font).unwrap();

        let game_width  = 64;
        let game_height = 30;

        let grid_view = GridView {
            rect:        Rect::new(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT),
            cell_size:   30,
            columns:     game_width,
            rows:        game_height,
            grid_origin: Point::new(0, 0),
            wrap:        true,
        };

        let mut color_settings = ColorSettings {
            cell_colors: BTreeMap::new(),
            background:  Color::RGB( 64,  64,  64),
        };
        color_settings.cell_colors.insert(CellState::Dead,  Color::RGB(255, 255, 255));
        color_settings.cell_colors.insert(CellState::Alive, Color::RGB(  0,   0,   0));
        color_settings.cell_colors.insert(CellState::Wall,  Color::RGB(158, 141, 105));
        color_settings.cell_colors.insert(CellState::Fog,   Color::RGB(128, 128, 128));

        let mut s = MainState {
            intro_text:          intro_text,
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 Universe::new(game_width, game_height).unwrap(),
            first_gen_was_drawn: false,
            grid_view:           grid_view,
            color_settings:      color_settings,
        };

        s.uni.set_word(0,16, 0x0000000000000003);
        s.uni.set_word(0,17, 0x0000000000000006);
        s.uni.set_word(0,18, 0x0000000000000002);

        Ok(s)
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) -> GameResult<()> {
        let duration = timer::duration_to_f64(dt); // seconds
        match self.stage {
            Stage::Intro(mut remaining) => {
                remaining -= duration;
                if remaining > 0.0 {
                    self.stage = Stage::Intro(remaining);
                } else {
                    self.stage = Stage::Run;
                }
            }
            Stage::Run => {
                println!("Gen: {}", self.uni.latest_gen());
                if self.first_gen_was_drawn {
                    self.uni.next();     // next generation
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        ctx.renderer.clear();
        match self.stage {
            Stage::Intro(_) => {
                try!(graphics::draw(ctx, &mut self.intro_text, None, None));
            }
            Stage::Run => {
                let rect1 = Rect::new(0, 0, 10, 10);
                let rect2 = Rect::new(20, 25, 10, 10);
                let rect_vec = vec![rect1, rect2];
                graphics::set_color(ctx, Color::RGB(0,255,0));
                graphics::rectangles(ctx,  graphics::DrawMode::Fill, &rect_vec).unwrap();
                graphics::set_color(ctx, Color::RGB(0,0,0));
                //XXX draw: need a libconway function that takes a closure and executes with coords of each alive cell (or an iterator???)

                //XXX will need a mapping between screen coords and game coords
                self.first_gen_was_drawn = true;
            }
        }
        ctx.renderer.present();
        timer::sleep_until_next_frame(ctx, FPS);
        Ok(())
    }

    fn mouse_button_down_event(&mut self, button: Mouse, x: i32, y: i32) {
        println!("Button down event! button:{:?} at ({}, {})", button, x, y);
    }
}


impl GridView {
    fn bounding_rect(&self) -> Rect {
        return self.rect;
    }

    fn game_coords_from_window(&self, point: Point) -> Option<(usize, usize)> {
        let mut col: isize = ((point.x() - self.grid_origin.x()) / self.cell_size) as isize;
        let mut row: isize = ((point.y() - self.grid_origin.y()) / self.cell_size) as isize;
        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            if self.wrap {
                col %= self.columns as isize;
                row %= self.rows as isize;
            } else {
                return None;
            }
        }
        Some((col as usize, row as usize))
    }

    // Attempt to return a rectangle for the on-screen area of the specified cell.
    // If partially in view, will be clipped by the bounding rectangle.
    // Caller must ensure that col and row are within bounds.
    fn window_coords_from_game(&self, col: usize, row: usize) -> Option<Rect> {
        let left   = self.grid_origin.x() + (col as i32) * self.cell_size;
        let right  = self.grid_origin.x() + (col + 1) as i32 * self.cell_size - 1;
        let top    = self.grid_origin.y() + (row as i32) * self.cell_size;
        let bottom = self.grid_origin.y() + (row + 1) as i32 * self.cell_size - 1;
        assert!(left < right);
        assert!(top < bottom);
        let rect = Rect::new(left, top, (right - left) as u32, (bottom - top) as u32);
        rect.intersection(self.rect)
    }
}


// Now our main function, which does three things:
//
// * First, create a new `ggez::conf::Conf`
// object which contains configuration info on things such
// as screen resolution and window title,
// * Second, create a `ggez::game::Game` object which will
// do the work of creating our MainState and running our game,
// * then just call `game.run()` which runs the `Game` mainloop.
pub fn main() {
    let mut c = conf::Conf::new();

    c.version       = version!().to_string();
    c.window_width  = SCREEN_WIDTH;
    c.window_height = SCREEN_HEIGHT;
    c.window_icon   = "conwaylife.ico".to_string();
    c.window_title  = "💥 ConWayste the Enemy 💥".to_string();

    // save conf to .toml file
    let mut f = File::create("aaron_conf.toml").unwrap(); //XXX
    c.to_toml_file(&mut f).unwrap();

    let mut game: Game<MainState> = Game::new("ConWaysteTheEnemy", c).unwrap();
    if let Err(e) = game.run() {
        println!("Error encountered: {:?}", e);
    } else {
        println!("Game exited cleanly.");
    }
}
