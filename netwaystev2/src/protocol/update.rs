use conway::universe::Region;

use serde::{Deserialize, Serialize};

// chat messages sent from server to all clients other than originating client
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BroadcastChatMessage {
    pub chat_seq:    Option<u64>, // Some(<number>) when sent to clients (starts at 0 for first
    // chat message sent to this client in this room); None when
    // internal to server
    pub player_name: String,
    pub message:     String, // should not contain newlines
}

// TODO: add support
// The server doesn't have to send all GameUpdates to all clients because that would entail keeping
// them all for the lifetime of the room, and sending that arbitrarily large list to clients upon
// joining.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum GameUpdate {
    GameNotification {
        msg: String,
    },
    GameStart {
        options: GameOptions,
    },
    PlayerList {
        /// List of names and other info of all users including current user.
        players: Vec<PlayerInfo>,
    },
    PlayerChange {
        /// Most up to date player information.
        player:   PlayerInfo,
        /// If there was a name change, this is the old name.
        old_name: Option<String>,
    },
    PlayerJoin {
        player: PlayerInfo,
    },
    PlayerLeave {
        name: String,
    },
    /// Game ended but the user is allowed to stay.
    GameFinish {
        outcome: GameOutcome,
    },
    /// Kicks user back to lobby. When it appears in an Update packet, it should be the last update
    /// in that packet.
    RoomDeleted,
    /// New match. Server suggests we join this room.
    /// NOTE: this is the only variant that can happen in a lobby.
    Match {
        room:        String,
        expire_secs: u32, // TODO: think about this
    },
}

// TODO: add support
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum UniUpdate {
    Diff { diff: GenStateDiffPart },
    NoChange,
}

// TODO: add support
/// One or more of these can be recombined into a GenStateDiff from the conway crate.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GenStateDiffPart {
    pub part_number:  u8,     // zero-based but less than 32
    pub total_parts:  u8,     // must be at least 1 but at most 32
    pub gen0:         u32,    // zero means diff is based off the beginning of time
    pub gen1:         u32,    // This is the generation when this diff has been applied.
    pub pattern_part: String, // concatenated together to form a Pattern
}

// TODO: add support
/// GenPartInfo is sent in the UpdateReply to indicate which GenStateDiffParts are needed.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GenPartInfo {
    pub gen0:         u32, // zero means diff is based off the beginning of time
    pub gen1:         u32, // must be greater than last_full_gen
    pub have_bitmask: u32, // bitmask indicating which parts for the specified diff are present; must be less than 1<<total_parts
}

// TODO: add support
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GameOutcome {
    pub winner: Option<String>, // Some(<name>) if winner, or None, meaning it was a tie/forfeit
}

/// All options needed to initialize a Universe. Notably, num_players is absent, because it can be
/// inferred from the index values of the latest list of PlayerInfos received from the server.
/// Also, is_server is absent.
// TODO: add support
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GameOptions {
    pub width:           u32,
    pub height:          u32,
    pub history:         u16,
    pub player_writable: Vec<NetRegion>,
    pub fog_radius:      u32,
}

/// Net-safe version of a libconway Region
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NetRegion {
    pub left:   i32,
    pub top:    i32,
    pub width:  u32,
    pub height: u32,
}

impl Into<Region> for NetRegion {
    fn into(self) -> Region {
        Region {
            left:   self.left as isize,
            top:    self.top as isize,
            width:  self.width as usize,
            height: self.height as usize,
        }
    }
}

// TODO: add support
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct PlayerInfo {
    /// Name of the player.
    pub name:  String,
    /// Index of player in Universe; None means this player is a lurker (non-participant)
    pub index: Option<u64>,
}
