#![allow(unused)] // XXX

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use snowflake::ProcessUniqueId;
use tracing::*;

const MAX_ROOM_NAME_CHARS: usize = 32;
const ROOMS_PER_SERVER: usize = 3;
const MODULE_NAME: &'static str = "Room";

// XXX PlayerId is defined here just for bring up. Might be best to relocate when ready.
type PlayerId = ProcessUniqueId;
type RoomId = ProcessUniqueId;

#[derive(Debug, thiserror::Error)]
pub enum RoomMgrError {
    #[error("Failed to create a new room, the server is full")]
    RoomBlockEmpty,
    #[error("A room with that name already exists: {name}")]
    RoomWithNameExists { name: String },
    #[error("Room does not exist: {id}")]
    RoomIdNotFound { id: RoomId },
    #[error("Invalid room name provided: {name}")]
    InvalidRoomName { name: String },
}

#[derive(Default)]
struct Room {
    name:     String,
    player_a: Option<PlayerId>,
    player_b: Option<PlayerId>,
}

pub struct RoomBlock {
    names2rid: IndexMap<String, RoomId>,
    rid2room:  IndexMap<RoomId, Room>,
    free_pool: Vec<Room>,
}

impl RoomBlock {
    pub fn new() -> RoomBlock {
        let mut room_pool = Vec::<Room>::with_capacity(ROOMS_PER_SERVER);
        for _ in 0..room_pool.capacity() {
            room_pool.push(Room::default());
        }

        RoomBlock {
            names2rid: IndexMap::new(),
            rid2room:  IndexMap::new(),
            free_pool: room_pool,
        }
    }

    pub fn count(&self) -> usize {
        assert!(self.rid2room.len() == self.names2rid.len());
        assert!(self.rid2room.len() == (ROOMS_PER_SERVER - self.free_pool.len()));

        self.rid2room.len()
    }

    pub fn alloc(&mut self, room_name: String) -> Result<RoomId> {
        trace!("[{}] allocating room => Name:{}", MODULE_NAME, room_name);

        if room_name == "" || room_name.len() > MAX_ROOM_NAME_CHARS {
            let error = RoomMgrError::InvalidRoomName { name: room_name };
            error!("[{}] {}", MODULE_NAME, error);
            return Err(anyhow!(error));
        }

        if self.free_pool.is_empty() {
            let error = RoomMgrError::RoomBlockEmpty;
            error!("[{}] {}", MODULE_NAME, error);
            return Err(anyhow!(error));
        }

        if self.names2rid.contains_key(&room_name) {
            let error = RoomMgrError::RoomWithNameExists { name: room_name };
            error!("[{}] {}", MODULE_NAME, error);
            return Err(anyhow!(error));
        }

        // Unwrap safe because of is_empty() check
        let mut room = self.free_pool.pop().unwrap();
        room.name = room_name.clone();

        let room_id = RoomId::new();

        trace!("[{}] room allocated => ID:{} Name:{} ", MODULE_NAME, room_id, room_name);

        // Return value ignored is okay because of contains_key() check
        let _ = self.names2rid.insert(room_name, room_id);

        // Return value ignored is okay the room ID is uniquely generated, and the room was pulled from the free pool
        let _ = self.rid2room.insert(room_id, room);

        Ok(room_id)
    }

    pub fn free(&mut self, room_id: RoomId) -> Result<()> {
        trace!("[{}] freeing room => ID:{}", MODULE_NAME, room_id);

        if let Some(room) = self.rid2room.remove(&room_id) {
            if let Some(_room_id) = self.names2rid.remove(&room.name) {
                trace!("[{}] room '{}' freed ", MODULE_NAME, room.name);
            } else {
                error!(
                    "[{}] failed to find room by name => ID:{} Name:{} ",
                    MODULE_NAME, room_id, room.name
                );
            }

            self.free_pool.push(Room::default());
        } else {
            return Err(anyhow!(RoomMgrError::RoomIdNotFound { id: room_id }));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::room::MAX_ROOM_NAME_CHARS;

    use super::{RoomBlock, RoomId, ROOMS_PER_SERVER};

    #[test]
    fn test_allocate_all_rooms() {
        let mut rooms = RoomBlock::new();

        for i in 0..ROOMS_PER_SERVER {
            assert!(rooms.alloc(format!("room {}", i)).is_ok());
        }
    }

    #[test]
    fn test_allocation_fails_pool_empty() {
        let mut rooms = RoomBlock::new();

        for i in 0..ROOMS_PER_SERVER {
            assert!(rooms.alloc(format!("room {}", i)).is_ok());

            let _ = rooms.count(); // Performs a size assertion internally
        }

        assert!(rooms.alloc("allocation-should-fail".into()).is_err());
    }

    #[test]
    fn test_allocation_fails_duplicate() {
        let mut rooms = RoomBlock::new();

        assert!(rooms.alloc("room-name".into()).is_ok());
        assert!(rooms.alloc("room-name".into()).is_err());
    }

    #[test]
    fn test_allocation_fails_invalid_name() {
        let mut rooms = RoomBlock::new();

        let long_name = "0".repeat(MAX_ROOM_NAME_CHARS + 1);

        assert!(rooms.alloc("".into()).is_err());
        assert!(rooms.alloc(long_name).is_err());
    }

    #[test]
    fn test_free_all() {
        let mut rooms = RoomBlock::new();

        let mut room_id_list = vec![];
        for i in 0..ROOMS_PER_SERVER {
            let room_id = rooms.alloc(format!("room {}", i)).expect("room allocation failed");
            room_id_list.push(room_id);
            let _ = rooms.count(); // Performs a size assertion internally
        }

        for r in room_id_list {
            assert!(rooms.free(r).is_ok());
            let _ = rooms.count(); // Performs a size assertion internally
        }
    }

    #[test]
    fn test_free_fails_unknown_room_id() {
        let mut rooms = RoomBlock::new();

        let renegade_room_id = RoomId::new();
        assert!(rooms.free(renegade_room_id).is_err());
    }
}
