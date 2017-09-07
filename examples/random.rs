extern crate conway;
extern crate rand;

use rand::Rng;
use std::{thread, time};
use conway::*;

fn main() {
        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];

        let mut uni = Universe::new(1280,  // width
                                    800,   // height
                                    true, // server_mode
                                    16,   // history
                                    2,    // players
                                    writable_regions,
                                    9     // fog radius
                                    ).unwrap();
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
