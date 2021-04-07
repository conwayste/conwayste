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
        endpoint:    Endpoint,
        amount:      usize,
        queue_index: isize,
    },
    SendPackets {
        endpoint: Endpoint,
        packets:  Vec<PacketInfo>,
    },
    DropEndpoint {
        endpoint: Endpoint,
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
    NoPacketAtIndex,
    BufferFull,
    ExceedsMtu,
    EndpointNotFound,
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

    /// The packet at this index in the tx queue of this endpoint has been resent the maximum number of times
    PacketTimeout {
        endpoint: Endpoint,
        index:    usize,
    },
}

#[derive(Debug)]
pub struct PacketInfo {
    packet:         Packet,
    retry_count:    u16,
    retry_interval: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
    /// PR_GATE Add additional fields here.
    data: [u8; 10],
}
