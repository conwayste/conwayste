extern crate conway;
extern crate rand;

use conway::universe::*;
use rand::Rng;
use std::{thread, time};

fn main() {
    let player0 = PlayerBuilder::new(Region::new(100, 70, 34, 16)); // used for the glider gun and predefined patterns
    let player1 = PlayerBuilder::new(Region::new(0, 0, 80, 80));
    let players = vec![player0, player1];

    let bigbang = BigBang::new()
        .width(1280)
        .height(800)
        .server_mode(true)
        .history(16)
        .fog_radius(6)
        .add_players(players)
        .birth();

    let mut uni = bigbang.unwrap();
    let step_time = time::Duration::from_millis(30);

    let mut rng = rand::thread_rng();
    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        let mut rand_word: u64 = rng.gen::<u8>() as u64;
        for col in 60..64 {
            for row in 15..17 {
                if rand_word & 1 == 1 {
                    let _ = uni.toggle(col, row, 0);
                }
                rand_word >>= 1;
            }
        }
        uni.next();
        thread::sleep(step_time);
    }
}
