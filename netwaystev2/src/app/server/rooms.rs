use anyhow::{anyhow, Result};

use crate::app::server::players::Player;
use crate::protocol::RoomStatus;

pub const ROOMS_PER_SERVER: usize = 3; // TODO: Move to config

#[derive(Default)]
pub struct Room {
    pub name:     String,
    pub player_a: Option<Player>,
    pub player_b: Option<Player>,
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
        self.rooms
            .iter()
            .map(|r| RoomStatus {
                in_progress:  false, // TODO: Get status from session state
                room_name:    r.name.to_owned(),
                player_count: r.player_a.as_ref().iter().chain(r.player_b.as_ref().iter()).count() as u8,
            })
            .collect()
    }
}
