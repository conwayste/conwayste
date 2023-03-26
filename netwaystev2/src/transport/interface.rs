use crate::common::Endpoint;
use crate::protocol::Packet;

use snowflake::ProcessUniqueId;

use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

// https://serverfault.com/questions/645890/tcpdump-truncates-to-1472-bytes-useful-data-in-udp-packets-during-the-capture/645892#645892
pub const UDP_MTU_SIZE: usize = 1472;

// Used for more informative event logging
#[derive(Debug, Clone, PartialEq)]
pub enum TransportMode {
    Client,
    Server,
}

impl Display for TransportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if *self == TransportMode::Client { "c" } else { "s" })
    }
}

/// Filter layer sends these commands to the Transport Layer to manage endpoints and their packets
#[derive(Debug, Clone)]
pub enum TransportCmd {
    NewEndpoint {
        endpoint: Endpoint,
        timeout:  Duration,
    },
    SendPackets {
        endpoint:     Endpoint,
        packet_infos: Vec<PacketSettings>,
        packets:      Vec<Packet>,
    },
    DropEndpoint {
        endpoint: Endpoint,
    },
    DropPacket {
        endpoint: Endpoint,
        tid:      ProcessUniqueId,
    },
    CancelTransmitQueue {
        endpoint: Endpoint,
    },
    Shutdown,
}

/// Transport layer sends these response codes for each Filter layer command (see `TransportCmd`)
#[derive(Debug, Clone)]
pub enum TransportRsp {
    Accepted,
    BufferFull,
    ExceedsMtu {
        tid:  ProcessUniqueId,
        size: usize,
        mtu:  usize,
    },
    EndpointError {
        error: Arc<anyhow::Error>,
    }, // Needs to be Arc to allow cloning
    SendPacketsLengthMismatch,
}

/// Used by the Transport layer to inform the Filter layer of a packet or endpoint event
#[derive(Debug, Clone)]
pub enum TransportNotice {
    /// Here is the received packet for this endpoint
    PacketDelivery { endpoint: Endpoint, packet: Packet },

    /// The maximum time since a packet was received from this endpoint was exceeded.
    EndpointTimeout { endpoint: Endpoint },

    /// It has been more than half the timeout time since a packet was received or sent. Once sent,
    /// it will not be sent again until after a packet was sent or received and sufficient time has
    /// passed, as described above.
    EndpointIdle { endpoint: Endpoint },
}

/// Used by the Filter layer to inform the Transport layer of packet settings
#[derive(Debug, Clone)]
pub struct PacketSettings {
    /// Transmit ID, a unique identifier used to sync packet transactions between the filter and Transport layers
    pub tid:            ProcessUniqueId,
    /// The length of time in between each retry attempt
    pub retry_interval: Duration,
}

#[allow(unused)] // ToDo: do we need this?
#[derive(Debug, thiserror::Error)]
pub enum TransportEndpointDataError {
    #[error("{endpoint:?} not found in transmit queue: {message}")]
    EndpointNotFound { endpoint: Endpoint, message: String },
    #[error("{endpoint:?} entry exists in transmit queue: {entry_found:?}")]
    EndpointExists {
        endpoint:    Endpoint,
        entry_found: Endpoint,
    },
    #[error("Transmit ID {tid} not found for {endpoint:?} in Transmit queue")]
    TransmitIDNotFound {
        endpoint: Endpoint,
        tid:      ProcessUniqueId,
    },
    #[error("Could not remove packet at index {index} from transmit queue with tid {tid} for {endpoint:?}")]
    PacketRemovalFailure {
        endpoint: Endpoint,
        tid:      ProcessUniqueId,
        index:    usize,
    },
    #[error("{endpoint:?} could not be dropped : {message}")]
    EndpointDropFailed { endpoint: Endpoint, message: String },
}
