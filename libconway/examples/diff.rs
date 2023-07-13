use std::env;

use rand::distributions::{Bernoulli, Distribution};
use rand::Rng;

use conway::universe::*;

const RANDOM_DENSITY: f64 = 0.4;

fn main() {
    let iterations: usize = env::args()
        .nth(1)
        .expect("iterations passed as first arg on cmd line")
        .parse()
        .expect("int");
    let player0 = PlayerBuilder::new(Region::new(100, 70, 34, 16));
    let player1 = PlayerBuilder::new(Region::new(0, 0, 80, 80));
    let players = vec![player0, player1];

    let bigbang = BigBang::new()
        .width(256)
        .height(128)
        .server_mode(true)
        .history(16)
        .fog_radius(6)
        .add_players(players)
        .birth();

    let mut uni = bigbang.unwrap();

    let mut rng = rand::thread_rng();

    let d = Bernoulli::new(RANDOM_DENSITY).unwrap();

    // Gen random pattern in player1 region
    for row in 0..50 {
        for col in 0..40 {
            let v = d.sample(&mut rng);
            if v {
                // Panics if not writable by this player but this is just a toy example :)
                uni.set(col, row, CellState::Alive(Some(1)), 1);
            }
        }
    }

    let mut gen0 = 0;
    for _ in 0..iterations {
        let gen1 = uni.latest_gen();
        println!("--\nGen: {}", uni.latest_gen());
        let gsd = uni.diff(gen0, gen1, Some(1)).expect("diff possible");
        describe_diff(gsd);
        uni.next();
        gen0 = gen1;
    }
}

fn describe_diff(gsd: GenStateDiff) {
    println!("Diff: {:?}", gsd);
    println!("Pattern size: {}", gsd.pattern.0.len());
    println!("Parts: {}", packets_per_pattern(gsd.pattern.0.len()));
}

// From netwaystev2/src/common.rs
const UDP_MTU_SIZE: usize = 1440;

// From netwaystev2/src/filter/server_update.rs
const MAX_GSDP_SIZE: usize = UDP_MTU_SIZE * 75 / 100;
const MAX_GSD_BYTES: usize = 32 * MAX_GSDP_SIZE; // ToDo: constantize the 32 (and combine with one in client_update.rs)

fn packets_per_pattern(bytesize: usize) -> usize {
    let mut packets = bytesize / MAX_GSDP_SIZE;
    let last_packet_fill_size = bytesize % MAX_GSDP_SIZE;
    if last_packet_fill_size > 0 {
        packets += 1;
    }
    packets
}
