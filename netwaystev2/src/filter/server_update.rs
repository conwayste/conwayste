use std::collections::VecDeque;
use std::sync::Arc;

use conway::GenStateDiff;

use crate::common::UDP_MTU_SIZE;
#[allow(unused)] // ToDo: need this?
use crate::protocol::{BroadcastChatMessage, GameUpdate, GenStateDiffPart};

// Most of OtherEndClient stuff should live here

/// Filter Layer's server-side representation of a room for a particular client.
#[allow(unused)]
#[derive(Debug)]
pub struct ServerRoom {
    room_name:        String,
    pub game_updates: GameUpdateQueue, // ToDo: consider removing `pub`
}

impl ServerRoom {
    pub fn new(room_name: String) -> Self {
        ServerRoom {
            room_name,
            game_updates: GameUpdateQueue::new(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum SplitGSDError {
    #[error("The size of this GenStateDiff is too large ({bytes:?} bytes > {max_bytes:?}) to break up into parts for sending")]
    DiffTooLarge {
        bytes:     usize,
        max_bytes: usize,
        parts:     usize,
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
        return Err(anyhow::anyhow!(SplitGSDError::DiffTooLarge {
            bytes,
            max_bytes: MAX_GSD_BYTES,
            parts: pattern_parts.len()
        }));
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

/// Keep track of acked GameUpdates; Note: this does not support wrapping sequence numbers, but
/// that should be fine.
#[derive(Debug)]
pub struct GameUpdateQueue {
    current_game_update_seq:     Option<u64>,
    last_acked_game_updated_seq: Option<u64>,
    unacked_game_updates:        VecDeque<(u64, GameUpdate)>, // front is oldest; back is newest
}

impl GameUpdateQueue {
    pub fn new() -> Self {
        GameUpdateQueue {
            current_game_update_seq:     None,
            last_acked_game_updated_seq: None,
            unacked_game_updates:        VecDeque::new(),
        }
    }

    pub fn push(&mut self, game_update: GameUpdate) {
        let seq = if let Some(oldseq) = self.current_game_update_seq {
            oldseq + 1
        } else {
            1
        };
        self.current_game_update_seq = Some(seq);
        self.unacked_game_updates.push_back((seq, game_update));
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.current_game_update_seq = None;
        self.last_acked_game_updated_seq = None;
        self.unacked_game_updates.clear();
    }

    pub fn len(&mut self) -> usize {
        self.unacked_game_updates.len()
    }

    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Get a Vec of all unacked GameUpdates, with their sequence numbers.
    ///
    /// Normally this call would be preceded by an ack(ack_from_client).
    pub fn get(&self) -> Vec<(u64, GameUpdate)> {
        self.unacked_game_updates.iter().cloned().collect()
    }

    /// Acknowledge none/some/all of these. Return true iff anything was acked.
    ///
    /// Normally this call would be followed with a get().
    pub fn ack(&mut self, acked_seq: Option<u64>) -> bool {
        let acked_seq = if let Some(acked) = acked_seq {
            acked
        } else {
            return false;
        };
        let prev_last = self.last_acked_game_updated_seq;
        self.last_acked_game_updated_seq = if let Some(last_acked_seq) = self.last_acked_game_updated_seq {
            Some(std::cmp::max(acked_seq, last_acked_seq))
        } else {
            Some(acked_seq)
        };
        if prev_last != self.last_acked_game_updated_seq {
            let mut acked_anything = false;
            loop {
                if let Some((seq, _)) = self.unacked_game_updates.front() {
                    if acked_seq < *seq {
                        break; // First unacked
                    }
                } else {
                    break; // Empty
                }
                self.unacked_game_updates.pop_front();
                acked_anything = true;
            }
            acked_anything
        } else {
            false
        }
    }
}

//XXX ServerGame
