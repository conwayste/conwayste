/// This generates Python code suitable for pasting into the Jupyter Notebooks in the
/// `nwv2-python-wrapper` folder.

use std::env;

use rand::distributions::{Bernoulli, Distribution};

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
        .add_players(players);

    let mut uni = bigbang.birth().unwrap();

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
    println!("gsds = [");
    for _ in 0..iterations {
        let gen1 = uni.latest_gen();
        println!("  # --\n  # Gen: {}", uni.latest_gen());
        let gsd = uni.diff(gen0, gen1, Some(1)).expect("diff possible");
        describe_diff(gsd);
        uni.next();
        gen0 = gen1;
    }
    println!("]\n# len(gsds) is {}", iterations);
    describe_bigbang(&bigbang);
}

fn describe_diff(gsd: GenStateDiff) {
    // Print in nwv2-python-wrapper format
    println!("  GenStateDiffW({}, {}, {:?}),", gsd.gen0, gsd.gen1, gsd.pattern.0);
    println!("  # Pattern size (uncompressed): {}", gsd.pattern.0.len());
}

fn describe_bigbang(bigbang: &BigBang) {
    println!("game_options = GameOptionsW(");
    println!("  {}, {}, {}, ", bigbang.width, bigbang.height, bigbang.history);
    println!("  [");
    for pw in &bigbang.player_writable {
        println!("    NetRegionW({}, {}, {}, {}),",
                 pw.left, pw.top, pw.width, pw.height);
    }
    println!("  ], {}", bigbang.fog_radius);
    println!(")\ngame_options");
}

/*
 * Commented the following because we use compression now.
// From netwaystev2/src/common.rs
const UDP_MTU_SIZE: usize = 1440;

// From netwaystev2/src/filter/server_update.rs
const MAX_GSDP_SIZE: usize = UDP_MTU_SIZE * 75 / 100;

fn packets_per_pattern(bytesize: usize) -> usize {
    let mut packets = bytesize / MAX_GSDP_SIZE;
    let last_packet_fill_size = bytesize % MAX_GSDP_SIZE;
    if last_packet_fill_size > 0 {
        packets += 1;
    }
    packets
}
*/
