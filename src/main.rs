extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128,32).unwrap();
    let step_time = time::Duration::from_millis(30);

    // pi heptomino
    uni.set_word(0,16, 0x0000000000000003);
    uni.set_word(0,17, 0x0000000000000006);
    uni.set_word(0,18, 0x0000000000000002);

    // Glider
    //uni.set_word(0,19, 0x0000000000000020);
    //uni.set_word(0,20, 0x0000000000000010);
    //uni.set_word(0,21, 0x0000000000000070);

    // Glider 2
    //uni.set_word(0,22, 0x0000006000000000);
    //uni.set_word(0,23, 0x0000003000000000);
    //uni.set_word(0,24, 0x0000004000000000);

    // Spaceship
    //uni.set_word(1,27, 0x0000000000000002);
    //uni.set_word(1,28, 0x0000000000000001);
    //uni.set_word(1,29, 0x0000000000000021);
    //uni.set_word(1,30, 0x000000000000001f);

    // Spaceship in reverse direction
    uni.set_word(1, 7, 0x0000000000040000);
    uni.set_word(1, 8, 0x0000000000080000);
    uni.set_word(1, 9, 0x0000000000084000);
    uni.set_word(1,10, 0x00000000000f8000);

    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
