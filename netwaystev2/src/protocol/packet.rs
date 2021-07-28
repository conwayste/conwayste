use serde::{Deserialize, Serialize};
use super::request::RequestAction;
use super::response::ResponseCode;
use super::update::{BroadcastChatMessage, GameUpdate, GenPartInfo, UniUpdate};
use crate::filter::PingPong;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
    Request {
        // sent by client
        sequence:     u64,
        response_ack: Option<u64>, // Next expected  sequence number the Server responds with to the Client.
        // Stated differently, the client has seen Server responses from 0 to response_ack-1.
        cookie:       Option<String>, // present if and only if action != connect
        action:       RequestAction,
    },
    Response {
        // sent by server in reply to client
        sequence:    u64,
        request_ack: Option<u64>, // most recent request sequence number received
        code:        ResponseCode,
    },
    Update {
        // Usually in-game: sent by server.
        // All of these except ping are reset to new values upon joining a room and cleared upon
        // leaving. Also note that the server may not send all GameUpdates or BroadcastChatMessages
        // in a single packet, since it could exceed the MTU.
        // TODO: limit chats and game_updates based on MTU!
        chats:           Vec<BroadcastChatMessage>, // All non-acknowledged chats are sent each update
        game_update_seq: Option<u64>,
        game_updates:    Vec<GameUpdate>, // Information pertaining to a game tick update.
        universe_update: UniUpdate,       // TODO: add support
        ping:            PingPong, // Used for server-to-client latency measurement (no room needed)
    },
    UpdateReply {
        // in-game: sent by client in reply to server
        cookie:               String,
        last_chat_seq:        Option<u64>, // sequence number of latest chat msg. received from server
        last_game_update_seq: Option<u64>, // seq. number of latest game update from server
        last_full_gen:        Option<u64>, // generation number client is currently at
        partial_gen:          Option<GenPartInfo>, // partial gen info, if some but not all GenStateDiffParts recv'd
        pong:                 PingPong, // Used for server-to-client latency measurement
    },
    GetStatus {
        ping: PingPong, // Used for client-to-server latency measurement
    },
    Status {
        pong:           PingPong, // used for client-to-server latency measurement
        server_version: String,
        player_count:   u64,
        room_count:     u64,
        server_name:    String,
        // TODO: max players?
    }, // Provide basic server information to the requester
}
