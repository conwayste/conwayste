extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128, 32, true, 16, 2, vec![Region::new(0,0,32,32), Region::new(96,0,32,32)]).unwrap();
    let step_time = time::Duration::from_millis(30);

    // pi heptomino
    uni.toggle(62, 16, 0);
    uni.toggle(63, 16, 0);
    uni.toggle(61, 17, 0);
    uni.toggle(62, 17, 0);
    uni.toggle(62, 18, 0);

    // Spaceship in reverse direction
    uni.toggle(45,  7, 1);
    uni.toggle(44,  8, 1);
    uni.toggle(44,  9, 1);
    uni.toggle(49,  9, 1);
    uni.toggle(44, 10, 1);
    uni.toggle(45, 10, 1);
    uni.toggle(46, 10, 1);
    uni.toggle(47, 10, 1);
    uni.toggle(48, 10, 1);

    loop {
        //println!("\x1b[H\x1b[2J{}", uni); //TODO: fix this
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
