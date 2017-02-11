extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128,32).unwrap();
    let step_time = time::Duration::from_millis(30);

    // pi heptomino
    uni.toggle(62, 16);
    uni.toggle(63, 16);
    uni.toggle(61, 17);
    uni.toggle(62, 17);
    uni.toggle(62, 18);

    // Spaceship in reverse direction
    uni.toggle(45,  7);
    uni.toggle(44,  8);
    uni.toggle(44,  9);
    uni.toggle(49,  9);
    uni.toggle(44, 10);
    uni.toggle(45, 10);
    uni.toggle(46, 10);
    uni.toggle(47, 10);
    uni.toggle(48, 10);

    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
