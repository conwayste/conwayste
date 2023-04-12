use std::{fmt::Display, num::Wrapping};

use conway::universe::GenStateDiff;

use super::ServerStatus;
use crate::{
    common::Endpoint,
    protocol::{BroadcastChatMessage, GameUpdate, RequestAction, ResponseCode},
};

/// App layer sends these commands to the Filter Layer to send game events to a peer
#[derive(Debug, Clone)]
pub enum FilterCmd {
    SendRequestAction {
        endpoint: Endpoint,
        action:   RequestAction,
    },
    SendResponseCode {
        endpoint: Endpoint,
        code:     ResponseCode,
    },
    SendChats {
        endpoints: Vec<Endpoint>,
        messages:  Vec<BroadcastChatMessage>,
    },
    SendGameUpdates {
        endpoints: Vec<Endpoint>,
        updates:   Vec<GameUpdate>,
    },
    CompleteAuthRequest {
        endpoint: Endpoint,
        decision: AuthDecision, // Subset of ResponseCode
    },
    ChangeServerStatus {
        // Keep this in sync with Packet::Status variant.
        server_version: Option<String>,
        player_count:   Option<u64>,
        room_count:     Option<u64>,
        server_name:    Option<String>,
    },
    SendGenStateDiff {
        endpoints: Vec<Endpoint>,
        diff:      GenStateDiff,
    },
    AddPingEndpoints {
        endpoints: Vec<Endpoint>,
    },
    ClearPingEndpoints,
    DropEndpoint {
        endpoint: Endpoint,
    },
    Shutdown {
        graceful: bool,
    },
}

/// Filter layer sends these responses to the Application Layer for each processed command
#[derive(Debug, Clone)]
pub enum FilterRsp {
    Accepted,
    NoSuchEndpoint { endpoint: Endpoint },
}

// TODO: consider removing the Vec from some of these (might not be needed if the transport layer
// isn't buffering things)
/// Used by the Filter layer to inform the Application layer of game update availability
#[derive(Debug, Clone)]
pub enum FilterNotice {
    HasGeneration {
        endpoints: Vec<Endpoint>,
        gen_num:   u64,
    },
    NewGenStateDiff {
        endpoint: Endpoint, // This is a server's endpoint; not much point in having this...
        diff:     GenStateDiff,
    },
    PingResult {
        endpoint:       Endpoint,
        latency:        Option<u64>,
        server_name:    String,
        server_version: String,
        room_count:     u64,
        player_count:   u64,
    },
    NewGameUpdates {
        endpoint: Endpoint, // This is a server's endpoint; not much point in having this...
        updates:  Vec<GameUpdate>,
    },
    NewChats {
        endpoint: Endpoint, // This is a server's endpoint; not much point in having this...
        messages: Vec<BroadcastChatMessage>,
    },
    NewRequestAction {
        endpoint: Endpoint,
        action:   RequestAction,
    },
    NewResponseCode {
        endpoint: Endpoint, // This is a server's endpoint; not much point in having this...
        code:     ResponseCode,
    },
    ClientAuthRequest {
        // At most one auth request outstanding per endpoint
        endpoint: Endpoint,
        fields:   ClientAuthFields,
    },
    EndpointTimeout {
        endpoint: Endpoint,
    },
}

pub type SeqNum = Wrapping<u64>;

#[derive(Debug, Clone)]
pub enum FilterMode {
    Client,
    Server(ServerStatus),
}

impl Display for FilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if self.is_client() { "c" } else { "s" })
        //write!(f, "c")
    }
}

impl FilterMode {
    pub fn is_client(&self) -> bool {
        use FilterMode::*;
        match self {
            Client => true,
            Server(_) => false,
        }
    }

    pub fn is_server(&self) -> bool {
        !self.is_client()
    }

    pub fn server_status(&self) -> Option<&ServerStatus> {
        use FilterMode::*;
        match self {
            Client => None,
            Server(ref status) => Some(status),
        }
    }

    pub fn server_status_mut(&mut self) -> Option<&mut ServerStatus> {
        use FilterMode::*;
        match self {
            Client => None,
            Server(ref mut status) => Some(status),
        }
    }
}

//XXX use
#[derive(Debug, Clone)]
pub enum AuthDecision {
    LoggedIn {
        cookie:         String,
        server_version: String,
    }, // player is logged in -- (cookie, server version)
    Unauthorized {
        error_msg: String,
    }, // 401 not logged in
}

impl Into<ResponseCode> for AuthDecision {
    fn into(self) -> ResponseCode {
        use AuthDecision::*;
        match self {
            LoggedIn { cookie, server_version } => ResponseCode::LoggedIn { cookie, server_version },
            Unauthorized { error_msg } => ResponseCode::Unauthorized { error_msg },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientAuthFields {
    pub player_name:    String,
    pub client_version: String,
    // ToDo: more here; whatever Filter layer knows that would help App layer make Auth decision
}
