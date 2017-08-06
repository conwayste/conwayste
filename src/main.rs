/*  Copyright 2017 the ConWaysteTheEnemy Developers.
 *
 *  This file is part of libconway.
 *
 *  libconway is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  libconway is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with libconway.  If not, see <http://www.gnu.org/licenses/>. */


extern crate conway;

use std::{thread, time};
use conway::*;

fn main() {
    let mut uni = Universe::new(128, 32, true, 16, 2, vec![Region::new(40,6,16,8), Region::new(60,16,8,8)], 16).unwrap();
    let step_time = time::Duration::from_millis(150);

    // pi heptomino
    uni.toggle(62, 17, 1).unwrap();
    uni.toggle(63, 17, 1).unwrap();
    uni.toggle(61, 18, 1).unwrap();
    uni.toggle(62, 18, 1).unwrap();
    uni.toggle(62, 19, 1).unwrap();

    // Spaceship in reverse direction
    uni.toggle(48, 6, 0).unwrap();
    uni.toggle(47, 7, 0).unwrap();
    uni.toggle(47, 8, 0).unwrap();
    uni.toggle(52, 8, 0).unwrap();
    uni.toggle(47, 9, 0).unwrap();
    uni.toggle(48, 9, 0).unwrap();
    uni.toggle(49, 9, 0).unwrap();
    uni.toggle(50, 9, 0).unwrap();
    uni.toggle(51, 9, 0).unwrap();

    uni.set_unchecked(74, 13, CellState::Wall);
    uni.set_unchecked(75, 13, CellState::Wall);
    uni.set_unchecked(76, 13, CellState::Wall);
    uni.set_unchecked(77, 13, CellState::Wall);
    uni.set_unchecked(78, 13, CellState::Wall);
    uni.set_unchecked(78, 14, CellState::Wall);
    uni.set_unchecked(78, 15, CellState::Wall);
    uni.set_unchecked(78, 16, CellState::Wall);
    uni.set_unchecked(78, 17, CellState::Wall);
    uni.set_unchecked(78, 18, CellState::Wall);
    uni.set_unchecked(78, 19, CellState::Wall);
    uni.set_unchecked(78, 20, CellState::Wall);
    uni.set_unchecked(78, 21, CellState::Wall);
    uni.set_unchecked(78, 22, CellState::Wall);
    uni.set_unchecked(78, 23, CellState::Wall);
    uni.set_unchecked(78, 24, CellState::Wall);
    uni.set_unchecked(77, 24, CellState::Wall);
    uni.set_unchecked(76, 24, CellState::Wall);
    uni.set_unchecked(75, 24, CellState::Wall);

    loop {
        println!("\x1b[H\x1b[2J{}", uni);
        println!("Gen: {}", uni.latest_gen());
        uni.next();
        thread::sleep(step_time);
    }
}
