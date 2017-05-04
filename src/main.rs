extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128, 32, true, 16, 2, vec![Region::new(40,6,16,8), Region::new(60,16,8,8)]).unwrap();
    let step_time = time::Duration::from_millis(30);

    // pi heptomino
    uni.toggle(62, 17, 1).unwrap();
    uni.toggle(63, 17, 1).unwrap();
    uni.toggle(61, 18, 1).unwrap();
    uni.toggle(62, 18, 1).unwrap();
    uni.toggle(62, 19, 1).unwrap();

    // Spaceship in reverse direction
    uni.toggle(48,  7, 0).unwrap();
    uni.toggle(47,  8, 0).unwrap();
    uni.toggle(47,  9, 0).unwrap();
    uni.toggle(52,  9, 0).unwrap();
    uni.toggle(47, 10, 0).unwrap();
    uni.toggle(48, 10, 0).unwrap();
    uni.toggle(49, 10, 0).unwrap();
    uni.toggle(50, 10, 0).unwrap();
    uni.toggle(51, 10, 0).unwrap();

    //XXX set_wall is just for testing
    uni.set_wall(26, 12);
    uni.set_wall(27, 12);
    uni.set_wall(28, 12);
    uni.set_wall(29, 12);
    uni.set_wall(30, 12);
    uni.set_wall(30, 13);
    uni.set_wall(30, 14);
    uni.set_wall(30, 15);
    uni.set_wall(30, 16);
    uni.set_wall(30, 17);
    uni.set_wall(30, 18);
    uni.set_wall(30, 19);
    uni.set_wall(30, 20);
    uni.set_wall(30, 21);
    uni.set_wall(30, 22);
    uni.set_wall(30, 23);
    uni.set_wall(29, 23);
    uni.set_wall(28, 23);
    uni.set_wall(27, 23);

    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
