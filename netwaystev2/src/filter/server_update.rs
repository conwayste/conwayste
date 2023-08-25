use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use lz4_flex::block::compress_prepend_size;

use conway::GenStateDiff;

use crate::common::UDP_MTU_SIZE;
#[allow(unused)] // ToDo: need this?
use crate::protocol::{BroadcastChatMessage, GameUpdate, GenStateDiffPart};

// Most of OtherEndClient stuff should live here

/// Filter Layer's server-side representation of a room for a particular client.
#[allow(unused)]
#[derive(Debug)]
pub struct ServerRoom {
    room_name:                 String,
    pub game_updates:          UnackedQueue<GameUpdate>, // ToDo: consider removing `pub`
    chats:                     UnackedQueue<ChatMessage>,
    pub latest_gen:            usize,
    pub latest_gen_client_has: usize,
    pub unacked_gsd_parts:     HashMap<(usize, usize), Vec<Option<Arc<GenStateDiffPart>>>>,
}

impl ServerRoom {
    pub fn new(room_name: String) -> Self {
        ServerRoom {
            room_name,
            game_updates: UnackedQueue::new(),
            chats: UnackedQueue::new(),
            latest_gen: 0,
            latest_gen_client_has: 0,
            unacked_gsd_parts: HashMap::new(),
        }
    }

    pub fn finish_game(&mut self) {
        self.latest_gen = 0;
        self.latest_gen_client_has = 0;
        self.unacked_gsd_parts.clear();
    }
}

/// This is server-internal and contains all fields in BroadcastChatMessage except chat_seq.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ChatMessage {
    player_name: String,
    message:     String, // should not contain newlines
}

impl ChatMessage {
    fn to_bcm(self, chat_seq: u64) -> BroadcastChatMessage {
        BroadcastChatMessage {
            chat_seq:    Some(chat_seq),
            player_name: self.player_name,
            message:     self.message,
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum SplitGSDError {
    #[error("The size of this GenStateDiff is too large ({byte_length:?} bytes > {max_bytes:?}) to break up into parts for sending")]
    DiffTooLarge {
        byte_length: usize,
        max_bytes:   usize,
        parts:       usize,
    },
}

/// Maximum size of a GenStateDiffPart's pattern component; 75% of MTU size to leave room for other stuff
const MAX_GSDP_SIZE: usize = UDP_MTU_SIZE * 75 / 100;
const MAX_GSD_BYTES: usize = 32 * MAX_GSDP_SIZE; // ToDo: constantize the 32 (and combine with one in client_update.rs)

/// Only possible error: SplitGSDError::DiffTooLarge
///
/// The "ok" return type is intended to be used for unacked_gsd_parts. The Vec corresponds to how
/// it's broken into packets. The Option allows setting to None when acked by client. The Arc is
/// for memory efficiency.
pub fn compress_and_split_gen_state_diff(diff: GenStateDiff) -> anyhow::Result<Vec<Option<Arc<GenStateDiffPart>>>> {
    let (gen0, gen1) = (diff.gen0 as u32, diff.gen1 as u32);
    let compressed_pattern = compress_prepend_size(diff.pattern.0.as_bytes());
    let byte_length = compressed_pattern.len();
    let pattern_parts = compressed_pattern
        .into_iter()
        .enumerate()
        .fold(Vec::<Vec<u8>>::new(), |mut v, (i, b)| {
            if i % MAX_GSDP_SIZE == 0 {
                let mut p = Vec::with_capacity(256);
                p.push(b);
                v.push(p);
            } else {
                v.last_mut().as_mut().unwrap().push(b);
            }
            v
        });
    if pattern_parts.len() > 32 {
        return Err(anyhow::anyhow!(SplitGSDError::DiffTooLarge {
            byte_length,
            max_bytes: MAX_GSD_BYTES,
            parts: pattern_parts.len()
        }));
    }
    let total_parts = pattern_parts.len() as u8;
    Ok(pattern_parts
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            Some(Arc::new(GenStateDiffPart {
                part_number: i as u8,
                total_parts,
                gen0,
                gen1,
                pattern_part: p,
            }))
        })
        .collect())
}

/// Keep track of acked GameUpdates and Chats; Note: this does not support wrapping sequence
/// numbers, but that should be fine.
#[derive(Debug)]
pub struct UnackedQueue<T> {
    current_seq:  Option<u64>,
    last_ack_seq: Option<u64>,
    unacked:      VecDeque<(u64, T)>, // front is oldest; back is newest
}

impl<T: Clone> UnackedQueue<T> {
    pub fn new() -> Self {
        UnackedQueue {
            current_seq:  None,
            last_ack_seq: None,
            unacked:      VecDeque::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        let seq = if let Some(oldseq) = self.current_seq {
            oldseq + 1
        } else {
            1
        };
        self.current_seq = Some(seq);
        self.unacked.push_back((seq, item));
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.current_seq = None;
        self.last_ack_seq = None;
        self.unacked.clear();
    }

    pub fn len(&mut self) -> usize {
        self.unacked.len()
    }

    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Get a Vec of all unacked items, with their sequence numbers.
    ///
    /// Normally this call would be preceded by an ack(ack_from_client).
    pub fn get(&self) -> Vec<(u64, T)> {
        self.unacked.iter().cloned().collect()
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
        let prev_last = self.last_ack_seq;
        self.last_ack_seq = if let Some(last_acked_seq) = self.last_ack_seq {
            Some(std::cmp::max(acked_seq, last_acked_seq))
        } else {
            Some(acked_seq)
        };
        if prev_last != self.last_ack_seq {
            let mut acked_anything = false;
            loop {
                if let Some((seq, _)) = self.unacked.front() {
                    if acked_seq < *seq {
                        break; // First unacked
                    }
                } else {
                    break; // Empty
                }
                self.unacked.pop_front();
                acked_anything = true;
            }
            acked_anything
        } else {
            false
        }
    }
}
