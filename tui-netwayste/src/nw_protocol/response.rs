use mimicry::*;
use mimicry::{MimicList, MimicFieldData, MimicMetadata};
use std::str::FromStr;
use strum_macros::{Display, EnumIter};

pub(crate) type PlayersVec = MimicList<String>;
pub(crate) type RoomsVec = MimicList<RoomList>;

#[derive(PartialEq, Debug, EnumIter, Display, Default, Mimic)]
pub(crate) enum ResponseCode {
    #[default]
    OK,
    LoggedIn {
        cookie:         String,
        server_version: String,
    },
    JoinedRoom {
        room_name: String,
    },
    LeaveRoom,
    PlayerList {
        players: PlayersVec,
    },
    RoomList {
        rooms: RoomsVec,
    },

    BadRequest {
        error_msg: String,
    },
    Unauthorized {
        error_msg: String,
    },
    TooManyRequests {
        error_msg: String,
    },
    ServerError {
        error_msg: String,
    },
    NotConnected {
        error_msg: String,
    },

    KeepAlive,
}

#[derive(Debug, PartialEq, Default)]
pub struct RoomList {
    pub(crate) room_name:    String,
    pub(crate) player_count: u8,
    // TODO: add support
    pub(crate) in_progress:  bool,
}

impl FromStr for RoomList {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let csv = s.split(",").map(|s: &str| s.trim()).collect::<Vec<&str>>();
        if csv.len() < 3 {
            return Err("Failed to parse RoomList, not enough parameters!");
        }
        if let Ok(room_name) = csv[0].parse::<String>() {
            if let Ok(player_count) = csv[1].parse::<u8>() {
                if let Ok(in_progress) = csv[2].parse::<bool>() {
                    return Ok(RoomList {
                        room_name,
                        player_count,
                        in_progress,
                    });
                } else {
                    return Err("Failed to parse in_progress in RoomList");
                }
            } else {
                return Err("Failed to parse player_count in RoomList");
            }
        } else {
            return Err("Failed to parse room name in RoomList");
        }
    }
}
