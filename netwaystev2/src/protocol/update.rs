use serde::{Deserialize, Serialize};

// chat messages sent from server to all clients other than originating client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastChatMessage {
    pub chat_seq:    Option<u64>, // Some(<number>) when sent to clients (starts at 0 for first
    // chat message sent to this client in this room); None when
    // internal to server
    pub player_name: String,
    pub message:     String, // should not contain newlines
}

impl BroadcastChatMessage {
    #[allow(unused)]
    pub fn new(sequence: u64, name: String, msg: String) -> BroadcastChatMessage {
        BroadcastChatMessage {
            chat_seq:    Some(sequence),
            player_name: name,
            message:     msg,
        }
    }

    fn sequence_number(&self) -> u64 {
        if let Some(v) = self.chat_seq {
            v
        } else {
            0
        }
    }
}