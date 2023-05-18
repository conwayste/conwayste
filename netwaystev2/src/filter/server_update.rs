use std::sync::Arc;

use snowflake::ProcessUniqueId;

use conway::GenStateDiff;

use crate::common::UDP_MTU_SIZE;
#[allow(unused)] // ToDo: need this?
use crate::protocol::{BroadcastChatMessage, GameUpdate, GenStateDiffPart};

// Most of OtherEndClient stuff should live here

/// Filter Layer's server-side representation of a room for a particular client.
#[allow(unused)]
#[derive(Debug)]
pub struct ServerRoom {
    room_name: String,
}

impl ServerRoom {
    pub fn new(room_name: String) -> Self {
        ServerRoom { room_name }
    }
}

#[derive(thiserror::Error, Debug)]
enum SplitGSDError {
    #[error("The size of this GenStateDiff is too large ({bytes:?} bytes > {max_bytes:?}) to break up into parts for sending")]
    DiffTooLarge {
        bytes: usize,
        max_bytes: usize,
        parts: usize,
    },
}

/// Maximum size of a GenStateDiffPart's pattern component; 75% of MTU size to leave room for other stuff
const MAX_GSDP_SIZE: usize = UDP_MTU_SIZE * 75 / 100;
const MAX_GSD_BYTES: usize = 32 * MAX_GSDP_SIZE;

/// Only possible error: SplitGSDError::DiffTooLarge
//XXX use
fn split_gen_state_diff(diff: GenStateDiff) -> anyhow::Result<Vec<Arc<GenStateDiffPart>>> {
    let (gen0, gen1) = (diff.gen0 as u32, diff.gen1 as u32);
    let bytes = diff.pattern.0.len();
    let pattern_parts = diff
        .pattern
        .0
        .chars()
        .enumerate()
        .fold(Vec::<String>::new(), |mut v, (i, c)| {
            if i % MAX_GSDP_SIZE == 0 {
                let mut s = String::with_capacity(256);
                s.push(c);
                v.push(s);
            } else {
                v.last_mut().as_mut().unwrap().push(c);
            }
            v
        });
    if pattern_parts.len() > 32 {
        return Err(anyhow::anyhow!(SplitGSDError::DiffTooLarge{bytes, max_bytes:MAX_GSD_BYTES, parts:pattern_parts.len()}));
    }
    let total_parts = pattern_parts.len() as u8;
    Ok(pattern_parts
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            Arc::new(GenStateDiffPart {
                part_number: i as u8,
                total_parts,
                gen0,
                gen1,
                pattern_part: p,
            })
        })
        .collect())
}

//XXX need this?
#[derive(Debug)]
pub struct GameUpdateQueue {
    current_game_update_seq:     Option<u64>,
    last_acked_game_updated_seq: Option<u64>,
    unacked_game_updates:        Vec<(u64, GameUpdate, ProcessUniqueId)>,
}

impl GameUpdateQueue {
    pub fn new() -> Self {
        GameUpdateQueue {
            current_game_update_seq:     None,
            last_acked_game_updated_seq: None,
            unacked_game_updates:        Vec::new(),
        }
    }

    //XXX
}

//XXX ServerGame
