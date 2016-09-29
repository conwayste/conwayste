extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128,32).unwrap();
    let step_time = time::Duration::from_millis(33);
    uni.set_word(0,16, 0x0000000000000003);
    uni.set_word(0,17, 0x0000000000000006);
    uni.set_word(0,18, 0x0000000000000002);

    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
