use anyhow::{anyhow, Result};
use snowflake::ProcessUniqueId;

use crate::protocol::RoomStatus;

const ROOMS_PER_SERVER: usize = 3; // TODO: Move to config

// XXX PlayerId is defined here just for bring up. Might be best to relocate when ready.
type PlayerId = ProcessUniqueId;

#[derive(Default)]
pub struct Room {
    pub name:     String,
    pub player_a: Option<PlayerId>,
    pub player_b: Option<PlayerId>,
}

impl Room {
    fn with_name(self, name: &str) -> Room {
        Room {
            name:     name.to_owned(),
            player_a: self.player_a,
            player_b: self.player_b,
        }
    }
}

pub struct ServerRooms {
    rooms: Vec<Room>,
}

impl ServerRooms {
    pub fn new() -> ServerRooms {
        let rooms = (0..ROOMS_PER_SERVER)
            .map(|i| Room::default().with_name(&format!("Room{}", i)))
            .collect();
        ServerRooms { rooms }
    }

    pub fn get_info(&self) -> Vec<RoomStatus> {
        self.rooms.iter().map(|r| RoomStatus {
            in_progress: false, // TODO: Get status from session state
            room_name: r.name.to_owned(),
            player_count: r.player_a.map_or_else(|| 0, |_| 1) + r.player_b.map_or_else(|| 0, |_| 1),
        }).collect()
    }
}
