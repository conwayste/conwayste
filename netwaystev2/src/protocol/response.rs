use serde::{Deserialize, Serialize};

// server response codes -- mostly inspired by https://en.wikipedia.org/wiki/List_of_HTTP_status_codes
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ResponseCode {
    // success - these are all 200 in HTTP
    // TODO: Many of these should contain the sequence number being acknowledged
    OK, // 200 no data
    LoggedIn {
        cookie:         String,
        server_version: String,
    }, // player is logged in -- (cookie, server version)
    JoinedRoom {
        room_name: String,
    }, // player has joined the room
    LeaveRoom, // player has left the room
    PlayerList {
        players: Vec<String>,
    }, // list of players in room or lobby
    RoomList {
        rooms: Vec<RoomList>,
    }, // list of rooms and their statuses

    // errors
    BadRequest {
        error_msg: String,
    }, // 400 unspecified error that is client's fault
    Unauthorized {
        error_msg: String,
    }, // 401 not logged in
    TooManyRequests {
        error_msg: String,
    }, // 429
    ServerError {
        error_msg: String,
    }, // 500
    NotConnected {
        error_msg: String,
    }, // no equivalent in HTTP due to handling at lower (TCP) level

    // Misc.
    KeepAlive, // Server's heart is beating
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct RoomList {
    pub room_name:    String,
    pub player_count: u8,
    // TODO: add support
    pub in_progress:  bool,
}
