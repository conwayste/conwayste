extern crate conway;
extern crate rand;

use rand::Rng;
use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128,32).unwrap();
    let step_time = time::Duration::from_millis(30);

    let mut rng = rand::thread_rng();
    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        let mut rand_word: u64 = rng.gen::<u8>() as u64;
        for col in 60..64 {
            for row in 15..17 {
                if rand_word & 1 == 1 {
                    uni.toggle(col, row);
                }
                rand_word >>= 1;
            }
        }
        uni.next();
        thread::sleep(step_time);
    }
}
