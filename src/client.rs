extern crate conway;
extern crate ggez;
#[macro_use]
extern crate version;

use ggez::conf;
use ggez::game::{Game, GameState};
use ggez::{GameResult, Context};
use ggez::graphics;
use ggez::timer;
use std::time::Duration;
use std::fs::File; //XXX ?
use conway::Universe;


const FPS: u32 = 30;
const INTRO_DURATION: f64 = 2.0;


#[derive(PartialEq)]
enum Stage {
    Intro(f64),   // seconds
    Run,          // TODO: break it out more to indicate whether waiting for game or playing game
}

// All game state
struct MainState {
    intro_text: graphics::Text,
    stage: Stage,
    uni: Universe,
    first_gen_was_drawn: bool,
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

        let mut s = MainState {
            intro_text: intro_text,
            stage: Stage::Intro(INTRO_DURATION),
            uni: Universe::new(128,32).unwrap(),
            first_gen_was_drawn: false,
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
                //XXX draw: need a libconway function that takes a closure and executes with coords of each (or an iterator???)

                //XXX will need a mapping between screen coords and game coords
                self.first_gen_was_drawn = true;
            }
        }
        ctx.renderer.present();
        timer::sleep_until_next_frame(ctx, FPS);
        Ok(())
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
    c.window_width  = 2000;
    c.window_height = 1200;
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
