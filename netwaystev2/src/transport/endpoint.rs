use super::interface::TransportEndpointDataError;
use crate::common::Endpoint;
use anyhow::{anyhow, Result};
use snowflake::ProcessUniqueId;

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Transport layers uses this to track packet-specific retries and timeouts
#[derive(Clone)]
struct PacketInfo {
    // FIXME: what does this timeout mean exactly (what happens when the timeout has been
    // exceeded)?
    transmit_interval: Duration,
    last_transmit:     Instant,
    retry_count:       usize,
}

impl PacketInfo {
    pub fn new(transmit_interval: Duration) -> Self {
        PacketInfo {
            transmit_interval,
            last_transmit: Instant::now(),
            retry_count: 0,
        }
    }
}

/// Transport layer uses this to determine if an endpoint is still active
struct EndpointMeta {
    endpoint_timeout: Duration,
    last_receive:     Option<Instant>,
}

impl EndpointMeta {
    pub fn new(timeout: Duration) -> Self {
        EndpointMeta {
            endpoint_timeout: timeout,
            last_receive:     None,
        }
    }
}

/// Used by the Transport layer to group a transmit id with the associated packet, for transmit
#[derive(Clone)]
struct PacketContainer<P> {
    tid:    ProcessUniqueId,
    packet: P,
    info:   PacketInfo,
}

impl<P> PacketContainer<P> {
    pub fn new(tid: ProcessUniqueId, packet: P, info: PacketInfo) -> Self {
        PacketContainer { tid, packet, info }
    }
}

/// The data for an endpoint, where P is the generic type of the thing to send (Packet).
pub(in crate::transport) struct TransportEndpointData<P> {
    endpoint_meta: HashMap<Endpoint, EndpointMeta>,
    transmit:      HashMap<Endpoint, VecDeque<PacketContainer<P>>>,
}

impl<P> TransportEndpointData<P> {
    pub fn new() -> Self {
        TransportEndpointData {
            endpoint_meta: HashMap::new(),
            transmit:      HashMap::new(),
        }
    }

    /// Create a new endpoint to transmit and receive data to and from.
    /// Will report an error if an entry for the endpoint already exists.
    pub fn new_endpoint(&mut self, endpoint: Endpoint, timeout: Duration) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::new());
            }
            Entry::Occupied(entry) => {
                return Err(anyhow!(TransportEndpointDataError::EndpointExists {
                    endpoint,
                    entry_found: *entry.key()
                }))
            }
        }

        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(EndpointMeta::new(timeout));
            }
            Entry::Occupied(entry) => {
                return Err(anyhow!(TransportEndpointDataError::EndpointExists {
                    endpoint,
                    entry_found: *entry.key()
                }))
            }
        }

        Ok(())
    }

    //XXX fix comment
    /// Enqueues data packets `item` to the received queue for the endpoint.
    ///
    /// If the endpoint does not exist, the Transport layer might be seeing a new connection.
    /// New connection has five seconds to complete authentication by a higher layer or it will be dropped.
    pub fn update_last_received(&mut self, endpoint: Endpoint) -> Result<()> {
        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(_) => self.new_endpoint(endpoint, Duration::from_secs(5))?,
            Entry::Occupied(mut entry) => entry.get_mut().last_receive = Some(Instant::now()),
        }
        Ok(())
    }

    /// Enqueues data packets `item` to the transmit queue for the endpoint. Each packet is assigned a transmit id (tid)
    /// by the Filter layer. Each packet may have a different timeout and retry limits depending on the semantic
    /// priority of the packet.
    /// Will report an error if the endpoint does not exist.
    pub fn push_transmit_queue(
        &mut self,
        endpoint: Endpoint,
        tid: ProcessUniqueId,
        item: P,
        transmit_interval: Duration,
    ) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(TransportEndpointDataError::EndpointNotFound {
                    endpoint,
                    message: format!("Failed to push packet with tid {}", tid),
                }));
            }
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .push_back(PacketContainer::new(tid, item, PacketInfo::new(transmit_interval)))
            }
        }

        Ok(())
    }

    /// Drops all data packets in a queue for the endpoint.
    /// Will report an error if the endpoint does not exist.
    pub fn clear_queue(&mut self, endpoint: Endpoint) -> Result<()> {
        if let Some(tx_queue) = self.transmit.get_mut(&endpoint) {
            tx_queue.clear()
        } else {
            return Err(anyhow!(TransportEndpointDataError::EndpointNotFound {
                endpoint,
                message: "Failed to clear queue".to_owned(),
            }));
        }
        Ok(())
    }

    /// Returns a vector of endpoints that have timed-out
    /// If the vector is empty, all endpoints still maintain active connections.
    pub fn timed_out_endpoints(&mut self) -> Vec<Endpoint> {
        let mut timed_out = vec![];
        for (endpoint, endpoint_meta) in &self.endpoint_meta {
            if let Some(last_receive) = endpoint_meta.last_receive {
                if Instant::now() - last_receive >= endpoint_meta.endpoint_timeout {
                    timed_out.push(*endpoint);
                }
            }
        }
        timed_out
    }

    /// Requested by the Filter layer to remove an endpoint.
    /// Will report an error if the endpoint does not exist.
    pub fn drop_endpoint(&mut self, endpoint: Endpoint) -> Result<()> {
        let mut invalid_endpoint = std::collections::HashSet::new();
        let mut error_message = String::new();

        if let None = self.transmit.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
            error_message.push_str("not found in transmit queue, ");
        }

        if let None = self.endpoint_meta.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
            error_message.push_str("not found in meta queue, ");
        }

        if !invalid_endpoint.is_empty() {
            Err(anyhow!(TransportEndpointDataError::EndpointDropFailed {
                endpoint,
                message: error_message,
            }))
        } else {
            Ok(())
        }
    }

    /// Requested by the Filter layer to remove a packet from the transmit queue. One use-case is if a packet is
    /// needs to be cancelled.
    /// Will report an error if the endpoint does not exist.
    /// Will report an error if the tid does not exist.
    /// Will report an error if the packet could not be removed.
    pub fn drop_packet(&mut self, endpoint: Endpoint, tid: ProcessUniqueId) -> Result<()> {
        let queue_index;
        if let Some(tx_queue) = self.transmit.get(&endpoint) {
            queue_index = tx_queue
                .iter()
                .position(|PacketContainer { tid: drop_tid, .. }| *drop_tid == tid);
        } else {
            return Err(anyhow!(TransportEndpointDataError::EndpointNotFound {
                endpoint,
                message: format!("Failed to drop packet with tid {}", tid),
            }));
        }

        if let Some(index) = queue_index {
            self.transmit.get_mut(&endpoint).unwrap().remove(index).map_or(
                Err(anyhow!(TransportEndpointDataError::PacketRemovalFailure {
                    endpoint,
                    tid,
                    index
                })),
                |_| Ok(()),
            )?;

            return Ok(());
        } else {
            return Err(anyhow!(TransportEndpointDataError::TransmitIDNotFound {
                endpoint,
                tid
            }));
        }
    }

    /// Returns a list of packets that can be retried across all endpoints.
    /// Side effect: updates last_transmit and retry_count on any packets that can be retried.
    pub fn retriable_packets(&mut self) -> Vec<(&P, Endpoint)> {
        let mut retry_qualified = vec![];

        for (endpoint, container) in &mut self.transmit {
            for PacketContainer { packet, info, .. } in container {
                // Add the packet to the list of retriable packets if enough time has passed since the last transmission,
                // and not all retries have been exhausted.
                if Instant::now() - info.last_transmit > info.transmit_interval {
                    info.last_transmit = Instant::now();
                    info.retry_count += 1;
                    retry_qualified.push((&*packet, *endpoint));
                }
            }
        }

        retry_qualified
    }
}
