use crate::common::Endpoint;

use serde::{Deserialize, Serialize};

use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub enum TransportQueueKind {
    Transmit,
    Receive,
}

#[derive(Debug)]
pub enum TransportCmd {
    NewEndpoint {
        endpoint: Endpoint,
        timeout:  Duration,
    },
    GetQueueCount {
        endpoint: Endpoint,
        kind:     TransportQueueKind,
    },
    TakeReceivePackets {
        endpoint: Endpoint,
    },
    SendPackets {
        endpoint:     Endpoint,
        packet_infos: Vec<PacketSettings>,
        // PR_GATE Change String to Packet
        packets:      Vec<String>,
    },
    DropEndpoint {
        endpoint: Endpoint,
    },
    DropPacket {
        endpoint: Endpoint,
        tid:      usize,
    },
    CancelTransmitQueue {
        endpoint: Endpoint,
    },
}

#[derive(Debug)]
pub enum TransportRsp {
    Accepted,
    TakenPackets {
        // PR_GATE Change String to Packet
        packets: Vec<String>,
    },
    QueueCount {
        endpoint: Endpoint,
        kind:     TransportQueueKind,
        count:    usize,
    },
    BufferFull,
    ExceedsMtu {
        tid: usize,
    },
    EndpointNotFound {
        endpoint: Endpoint,
    },
    SendPacketsLengthMismatch,
}

#[derive(Debug)]
pub enum TransportNotice {
    /// There are packets available on this endpoint
    PacketsAvailable {
        endpoint: Endpoint,
    },

    // The maximum time since a packet was received from this endpoint was exceeded.
    EndpointTimeout {
        endpoint: Endpoint,
    },

    /// A packet in the tx queue for this endpoint has been resent the maximum number of times
    PacketTimeout {
        endpoint: Endpoint,
        tid:      usize,
    },
}

/// Used by the filter layer to inform the transport layer of packet settings
#[derive(Debug)]
pub struct PacketSettings {
    /// Transmit ID, a unique identifier used to sync packet transactions between the filter and transport layers
    pub tid:            usize,
    /// The maximum number of retries for a Packet
    pub retry_limit:    usize,
    /// The length of time in between each retry attempt
    pub retry_interval: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
    /// PR_GATE Add additional fields here.
    data: [u8; 10],
}
