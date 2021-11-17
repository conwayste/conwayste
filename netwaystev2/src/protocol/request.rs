use serde::{Deserialize, Serialize};

////////////////////// Data model ////////////////////////
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum RequestAction {
    None, // never actually sent

    /* These actions do not require a user to be logged in to the server */
    Connect {
        name:           String,
        client_version: String,
    },

    /* All actions below require a log-in via a Connect request */
    Disconnect,
    KeepAlive {
        latest_response_ack: u64,
    }, // Send latest response ack on each heartbeat
    ListPlayers,
    ChatMessage {
        message: String,
    },
    ListRooms,
    NewRoom {
        room_name: String,
    },
    JoinRoom {
        room_name: String,
    },
    LeaveRoom,
    // TODO: add support ("auto_match" bool key, see issue #101)
    SetClientOptions {
        key:   String,
        value: Option<ClientOptionValue>,
    },
    // TODO: add support
    // Draw the specified RLE Pattern with upper-left cell at position x, y.
    DropPattern {
        x:       i32,
        y:       i32,
        pattern: String,
    },
    // TODO: add support (also need it in the ggez client)
    // Clear all cells in the specified region not belonging to other players. No part of this
    // region may be outside the player's writable region.
    ClearArea {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ClientOptionValue {
    Bool { value: bool },
    U8 { value: u8 },
    U16 { value: u16 },
    U32 { value: u32 },
    U64 { value: u64 },
    I8 { value: i8 },
    I16 { value: i16 },
    I32 { value: i32 },
    I64 { value: i64 },
    Str { value: String },
    List { value: Vec<ClientOptionValue> },
}