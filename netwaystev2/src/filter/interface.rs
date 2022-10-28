use std::num::Wrapping;

use conway::universe::GenStateDiff;

use crate::{
    common::Endpoint,
    protocol::{BroadcastChatMessage, GameUpdate, GenStateDiffPart, RequestAction, ResponseCode},
};

pub type SeqNum = Wrapping<u64>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum FilterMode {
    Client,
    Server,
}

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
        messages:  Vec<GameUpdate>,
    },
    Authenticated {
        endpoint: Endpoint,
    },
    SendGenStateDiff {
        endpoints: Vec<Endpoint>,
        diff:      GenStateDiffPart,
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
#[derive(Debug)]
pub enum FilterRsp {
    Accepted,
    NoSuchEndpoint { endpoint: Endpoint },
}

// TODO: consider removing the Vec from some of these (might not be needed if the transport layer
// isn't buffering things)
/// Used by the Filter layer to inform the Application layer of game update availability
#[derive(Debug)]
pub enum FilterNotice {
    HasGeneration {
        endpoints: Vec<Endpoint>,
        gen_num:   u64,
    },
    NewGenStateDiff {
        endpoint: Endpoint,
        diff:     GenStateDiff,
    },
    PingResult {
        endpoint:       Endpoint,
        latency:        u64,
        server_name:    String,
        server_version: String,
        room_count:     u64,
        player_count:   u64,
    },
    NewGameUpdates {
        endpoint: Endpoint,
        updates:  Vec<GameUpdate>,
    },
    NewChats {
        endpoint: Endpoint,
        messages: Vec<BroadcastChatMessage>,
    },
    NewRequestAction {
        endpoint: Endpoint,
        action:   RequestAction,
    },
    NewResponseCode {
        endpoint: Endpoint,
        code:     ResponseCode,
    },
    EndpointTimeout {
        endpoint: Endpoint,
    },
}
