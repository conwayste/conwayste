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

//XXX ServerGame
