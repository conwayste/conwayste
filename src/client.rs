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


const FPS: u32 = 25;
const INTRO_DURATION: f64 = 2.0;
const SCREEN_WIDTH: u32 = 2000;
const SCREEN_HEIGHT: u32 = 1200;
const PIXELS_SCROLLED_PER_FRAME: i32 = 50;


#[derive(PartialEq)]
enum Stage {
    Intro(f64),   // seconds
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
}

// All game state
struct MainState {
    small_font:          graphics::Font,
    intro_text:          graphics::Text,
    stage:               Stage,             // What state we are in (Intro/Menu Main/Generations..)
    uni:                 Universe,          // Things alive and moving here
    first_gen_was_drawn: bool,              // the purpose of this is to inhibit gen calc until the first draw
    grid_view:           GridView,          // 
    color_settings:      ColorSettings,
    running:             bool,

    // Input state
    single_step:         bool,
    arrow_input:         (i32, i32),
    drag_draw:           Option<CellState>,
}

// Support non-alive/dead/bg colors
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


// Then we implement the `ggez::game::GameState` trait on it, which
// requires callbacks for creating the game state, updating it each
// frame, and drawing it.
//
// The `GameState` trait also contains callbacks for event handling
// that you can override if you wish, but the defaults are fine.
impl GameState for MainState {
    fn load(ctx: &mut Context, _conf: &conf::Conf) -> GameResult<MainState> {
        let intro_font = graphics::Font::new(ctx, "DejaVuSerif.ttf", 48).unwrap();
        let intro_text = graphics::Text::new(ctx, "WAYSTE EM!", &intro_font).unwrap();

        let game_width  = 64*4;
        let game_height = 30*4;

        let grid_view = GridView {
            rect:        Rect::new(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT),
            cell_size:   30,
            columns:     game_width,
            rows:        game_height,
            grid_origin: Point::new(0, 0),
        };

        let mut color_settings = ColorSettings {
            cell_colors: BTreeMap::new(),
            background:  Color::RGB( 64,  64,  64),
        };
        color_settings.cell_colors.insert(CellState::Dead,  Color::RGB(224, 224, 224));
        color_settings.cell_colors.insert(CellState::Alive, Color::RGB(  0,   0,   0));
        color_settings.cell_colors.insert(CellState::Wall,  Color::RGB(158, 141, 105));
        color_settings.cell_colors.insert(CellState::Fog,   Color::RGB(128, 128, 128));

        let small_font = graphics::Font::new(ctx, "DejaVuSerif.ttf", 20).unwrap();
        let mut s = MainState {
            small_font:          small_font,
            intro_text:          intro_text,
            stage:               Stage::Intro(INTRO_DURATION),
            uni:                 Universe::new(game_width, game_height).unwrap(),
            first_gen_was_drawn: false,
            grid_view:           grid_view,
            color_settings:      color_settings,
            running:             false,
            single_step:         false,
            arrow_input:         (0, 0),
            drag_draw:           None,
        };

        // Initialize patterns
        /*
        // R pentomino
        s.uni.toggle(16, 15);
        s.uni.toggle(17, 15);
        s.uni.toggle(15, 16);
        s.uni.toggle(16, 16);
        s.uni.toggle(16, 17);
        */

        /*
        // Acorn
        s.uni.toggle(23, 19);
        s.uni.toggle(24, 19);
        s.uni.toggle(24, 17);
        s.uni.toggle(26, 18);
        s.uni.toggle(27, 19);
        s.uni.toggle(28, 19);
        s.uni.toggle(29, 19);
        */


        // Simkin glider gun
        s.uni.toggle(100, 70);
        s.uni.toggle(100, 71);
        s.uni.toggle(101, 70);
        s.uni.toggle(101, 71);

        s.uni.toggle(104, 73);
        s.uni.toggle(104, 74);
        s.uni.toggle(105, 73);
        s.uni.toggle(105, 74);

        s.uni.toggle(107, 70);
        s.uni.toggle(107, 71);
        s.uni.toggle(108, 70);
        s.uni.toggle(108, 71);

        /* eater
        s.uni.toggle(120, 87);
        s.uni.toggle(120, 88);
        s.uni.toggle(121, 87);
        s.uni.toggle(121, 89);
        s.uni.toggle(122, 89);
        s.uni.toggle(123, 89);
        s.uni.toggle(123, 90);
        */

        s.uni.toggle(121, 80);
        s.uni.toggle(121, 81);
        s.uni.toggle(121, 82);
        s.uni.toggle(122, 79);
        s.uni.toggle(122, 82);
        s.uni.toggle(123, 79);
        s.uni.toggle(123, 82);
        s.uni.toggle(125, 79);
        s.uni.toggle(126, 79);
        s.uni.toggle(126, 83);
        s.uni.toggle(127, 80);
        s.uni.toggle(127, 82);
        s.uni.toggle(128, 81);

        s.uni.toggle(131, 81);
        s.uni.toggle(131, 82);
        s.uni.toggle(132, 81);
        s.uni.toggle(132, 82);



        Ok(s)
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) -> GameResult<()> {
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
                if self.single_step {
                    self.running = false;
                }
                if self.first_gen_was_drawn && (self.running || self.single_step) {
                    self.uni.next();     // next generation
                    self.single_step = false;
                }
                if self.arrow_input != (0, 0) {
                    let (dx, dy) = self.arrow_input;
                    self.grid_view.grid_origin = self.grid_view.grid_origin.offset(-dx * PIXELS_SCROLLED_PER_FRAME,
                                                                                   -dy * PIXELS_SCROLLED_PER_FRAME);
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
                ////////// draw universe
                // grid background
                graphics::set_color(ctx, self.color_settings.get_color(None));
                graphics::rectangle(ctx,  graphics::DrawMode::Fill, self.grid_view.rect).unwrap();

                // grid foreground (dead cells)
                //TODO: put in its own function (of GridView); also make this less ugly
                let origin = self.grid_view.grid_origin;
                let full_width  = self.grid_view.grid_width();
                let full_height = self.grid_view.grid_height();
                let full_rect = Rect::new(origin.x(), origin.y(), full_width, full_height);
                if let Some(clipped_rect) = full_rect.intersection(self.grid_view.rect) {
                    graphics::set_color(ctx, self.color_settings.get_color(Some(CellState::Dead)));
                    graphics::rectangle(ctx,  graphics::DrawMode::Fill, clipped_rect).unwrap();
                }

                // grid non-dead cells
                self.uni.each_non_dead_full(&mut |col, row, state| {
                    let color = self.color_settings.get_color(Some(state));
                    graphics::set_color(ctx, color);

                    if let Some(rect) = self.grid_view.window_coords_from_game(col, row) {
                        graphics::rectangle(ctx,  graphics::DrawMode::Fill, rect).unwrap();
                    }
                });


                ////////// draw generation counter
                let gen_counter_str = self.uni.latest_gen().to_string();
                let mut gen_counter_text = graphics::Text::new(ctx,
                                                               &gen_counter_str,
                                                               &self.small_font).unwrap();
                let dst = Rect::new(0, 0, gen_counter_text.width(), gen_counter_text.height());
                graphics::draw(ctx, &mut gen_counter_text, None, Some(dst))?;


                ////////////////////// END
                graphics::set_color(ctx, Color::RGB(0,0,0)); // do this at end; not sure why...?
                self.first_gen_was_drawn = true;
            }
        }
        ctx.renderer.present();
        timer::sleep_until_next_frame(ctx, FPS);
        Ok(())
    }

    fn mouse_button_down_event(&mut self, button: Mouse, x: i32, y: i32) {
        if button == Mouse::Left {
            if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                let state = self.uni.toggle(col, row);
                self.drag_draw = Some(state);
            }
        }
    }

    fn mouse_motion_event(&mut self, state: MouseState, x: i32, y: i32, _xrel: i32, _yrel: i32) {
        if state.left() && self.drag_draw != None {
            if let Some((col, row)) = self.grid_view.game_coords_from_window(Point::new(x,y)) {
                let cell_state = self.drag_draw.unwrap();
                self.uni.set(col, row, cell_state);
            }
        }
    }

    fn mouse_button_up_event(&mut self, _button: Mouse, _x: i32, _y: i32) {
        // Later, we'll need to support drag-and-drop patterns as well as drag draw
        self.drag_draw = None;   // probably unnecessary because of state.left() check in mouse_motion_event
    }

    fn key_down_event(&mut self, opt_keycode: Option<Keycode>, _keymod: Mod, repeat: bool) {
        if opt_keycode == None {
            println!("WARNING: got opt_keycode of None; what could this mean???");
            return;
        }
        let keycode = opt_keycode.unwrap();

        match self.stage {
            Stage::Intro(_) => {
                self.stage = Stage::Run;
            }
            Stage::Run => {
                match keycode {
                    Keycode::Return => {
                        if !repeat {
                            self.running = !self.running;
                        }
                    }
                    Keycode::Space => {
                        self.single_step = true;
                    }
                    Keycode::Up => {
                        self.arrow_input = (0, -1);
                    }
                    Keycode::Down => {
                        self.arrow_input = (0,  1);
                    }
                    Keycode::Left => {
                        self.arrow_input = (-1, 0);
                    }
                    Keycode::Right => {
                        self.arrow_input = ( 1, 0);
                    }
                    Keycode::Plus | Keycode::Equals => {
                        // Zoom In
                        if self.grid_view.cell_size < 100 { // do we need a max
                            self.grid_view.cell_size += 1;
                        }
                    }
                    Keycode::Minus | Keycode::Underscore => {
                        // Zoom Out
                        if self.grid_view.cell_size > 0 {
                            self.grid_view.cell_size -= 1;
                        }
                    }
                    _ => {
                        println!("Unrecognized keycode {}", keycode);
                    }
                }
            }
        }
    }

    fn key_up_event(&mut self, opt_keycode: Option<Keycode>, _keymod: Mod, _repeat: bool) {
        if opt_keycode == None {
            println!("WARNING: got opt_keycode of None; what could this mean???");
            return;
        }
        let keycode = opt_keycode.unwrap();

        match keycode {
            Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right => {
                self.arrow_input = (0, 0);
            }
            _ => {}
        }
    }
}


// Controls the mapping between window and game coordinates
struct GridView {
    rect:        Rect,  // the area the game grid takes up on screen
    cell_size:   i32,   // zoom level in window coordinates
    columns:     usize, // width in game coords (should match bitmap/universe width)
    rows:        usize, // height in game coords (should match bitmap/universe height)
    grid_origin: Point, // top-left corner of grid w.r.t window coords. (may be outside rect)
}


impl GridView {
    // Returns Option<(col, row)>
    fn game_coords_from_window(&self, point: Point) -> Option<(usize, usize)> {
        let col: isize = ((point.x() - self.grid_origin.x()) / self.cell_size) as isize;
        let row: isize = ((point.y() - self.grid_origin.y()) / self.cell_size) as isize;
        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some((col as usize, row as usize))
    }

    // Attempt to return a rectangle for the on-screen area of the specified cell.
    // If partially in view, will be clipped by the bounding rectangle.
    // Caller must ensure that col and row are within bounds.
    fn window_coords_from_game(&self, col: usize, row: usize) -> Option<Rect> {
        let left   = self.grid_origin.x() + (col as i32)     * self.cell_size;
        let right  = self.grid_origin.x() + (col + 1) as i32 * self.cell_size - 1;
        let top    = self.grid_origin.y() + (row as i32)     * self.cell_size;
        let bottom = self.grid_origin.y() + (row + 1) as i32 * self.cell_size - 1;
        assert!(left < right);
        assert!(top < bottom);
        let rect = Rect::new(left, top, (right - left) as u32, (bottom - top) as u32);
        rect.intersection(self.rect)
    }

    fn grid_width(&self) -> u32 {
        self.columns as u32 * self.cell_size as u32
    }

    fn grid_height(&self) -> u32 {
        self.rows as u32 * self.cell_size as u32
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
