use crate::settings::VERSION;
use crate::{filter::PingPong, protocol::Packet};

#[derive(Debug, Clone)]
pub struct ServerStatus {
    // keep in sync with Packet::Status variant.
    pub server_version: String,
    pub player_count:   u64,
    pub room_count:     u64,
    pub server_name:    String,
}

impl Default for ServerStatus {
    fn default() -> Self {
        ServerStatus {
            server_version: VERSION.into(),
            player_count:   0,
            room_count:     0,
            server_name:    "unspecified server name".into(),
        }
    }
}

impl ServerStatus {
    pub fn to_packet(&self, ping: PingPong) -> Packet {
        Packet::Status {
            pong:           ping,
            server_version: self.server_version.clone(),
            player_count:   self.player_count,
            room_count:     self.room_count,
            server_name:    self.server_name.clone(),
        }
    }
}
