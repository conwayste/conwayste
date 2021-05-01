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
        packet_infos: Vec<PacketInfo>,
        packets:      Vec<Packet>,
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
    UnknownPacketTid,
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

    /// A packet in the tx queue for this endpoint has been resent the maximum number of times
    PacketTimeout {
        endpoint: Endpoint,
        tid:      usize,
    },
}

#[derive(Debug)]
pub struct PacketInfo {
    tid:            usize,
    retry_count:    u16,
    retry_interval: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
    /// PR_GATE Add additional fields here.
    data: [u8; 10],
}
