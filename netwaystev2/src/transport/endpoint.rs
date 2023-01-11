use super::interface::TransportEndpointDataError;
use super::TransportRsp;
use crate::common::Endpoint;
use crate::settings::TRANSPORT_RETRY_COUNT_LOG_THRESHOLD;
use anyhow::{anyhow, Result};
use snowflake::ProcessUniqueId;

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

const NEW_PACKET_ENDPOINT_TIMEOUT: Duration = Duration::from_secs(5);

/// Transport layers uses this to track packet-specific retries and timeouts.
///
/// The transmit interval describes the minimum time between packets retries. Each retry will consume one attempt every
/// transmit interval. Packets retries are tracked using the retry count and will incur a log statement after surpassing
/// `TRANSPORT_RETRY_COUNT_LOG_THRESHOLD`.
///
/// A transmit interval or retry count equal to zero indicates the packet will only be sent once on the initial
/// transmission. No retries are attempted. A non-zero transmit interval indicates the period between each retry.
#[derive(Clone)]
struct PacketInfo {
    transmit_interval: Duration,
    last_transmit:     Instant,
    retry_count:       usize,
    retry_logged:      bool,
}

impl PacketInfo {
    pub fn new(transmit_interval: Duration) -> Self {
        PacketInfo {
            transmit_interval,
            last_transmit: Instant::now(),
            retry_count: 0,
            retry_logged: false,
        }
    }
}

/// Transport layer uses this to determine if an endpoint is still active
struct EndpointMeta {
    endpoint_timeout:    Duration,
    last_receive:        Option<Instant>,
    last_send:           Option<Instant>,
    notified_of_timeout: bool,
    notified_of_idle:    bool,
}

impl EndpointMeta {
    pub fn new(timeout: Duration) -> Self {
        EndpointMeta {
            endpoint_timeout:    timeout,
            last_receive:        None,
            last_send:           None,
            notified_of_timeout: false,
            notified_of_idle:    false,
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
    pub fn new_endpoint(&mut self, endpoint: Endpoint, timeout: Duration) -> Result<TransportRsp> {
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
                self.transmit.remove(&endpoint); // Probably unreachable, but just in case -- remove what we inserted above.
                return Err(anyhow!(TransportEndpointDataError::EndpointExists {
                    endpoint,
                    entry_found: *entry.key()
                }));
            }
        }

        Ok(TransportRsp::Accepted)
    }

    /// Updates the last received time for the given endpoint. If the endpoint does not exist, a
    /// new one is created. This should be called when a new packet arrives.
    pub fn update_last_received(&mut self, endpoint: Endpoint) -> Result<()> {
        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(_) => {
                self.new_endpoint(endpoint, NEW_PACKET_ENDPOINT_TIMEOUT)?;
            }
            Entry::Occupied(mut entry) => {
                let meta = entry.get_mut();
                meta.last_receive = Some(Instant::now());
                meta.notified_of_idle = false;
            }
        }
        Ok(())
    }

    /// Updates the last sent time for the given endpoint. This should be called when a packet is
    /// sent.
    pub fn update_last_sent(&mut self, endpoint: Endpoint) -> Result<()> {
        if let Some(meta) = self.endpoint_meta.get_mut(&endpoint) {
            meta.last_send = Some(Instant::now());
            meta.notified_of_idle = false;
            Ok(())
        } else {
            Err(anyhow!(TransportEndpointDataError::EndpointNotFound {
                endpoint,
                message: "Cannot update last_send".into(),
            }))
        }
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

    /// Returns a vector of endpoints that have timed-out and have not resulted in TransportNotice.
    /// If the vector is empty, all endpoints still maintain active connections.
    pub fn timed_out_endpoints_needing_notify(&mut self) -> Vec<Endpoint> {
        let mut timed_out_unnotified = vec![];
        for (endpoint, endpoint_meta) in &self.endpoint_meta {
            // Exclude endpoints that we have notified about
            if endpoint_meta.notified_of_timeout {
                continue;
            }
            if let Some(last_receive) = endpoint_meta.last_receive {
                if Instant::now() - last_receive >= endpoint_meta.endpoint_timeout {
                    timed_out_unnotified.push(*endpoint);
                }
            }
        }
        timed_out_unnotified
    }

    /// Indicate that an "endpoint timed out" TransportNotice for this Endpoint has been sent.
    /// Returns whether an un-timed out entry was found and marked as timed out.
    pub fn mark_endpoint_as_timeout_notified(&mut self, endpoint: Endpoint) -> bool {
        if let Some(endpoint_meta) = self.endpoint_meta.get_mut(&endpoint) {
            // Return false if already marked as timed out
            if endpoint_meta.notified_of_timeout {
                return false;
            }
            endpoint_meta.notified_of_timeout = true;
            true
        } else {
            false
        }
    }

    /// Collect a Vec of all endpoints needing an EndpointIdle notify.
    pub fn idle_endpoints_needing_notify(&mut self) -> Vec<Endpoint> {
        let mut idle_unnotified = vec![];
        for (endpoint, endpoint_meta) in &self.endpoint_meta {
            // Exclude endpoints that we have notified about
            if endpoint_meta.notified_of_idle {
                continue;
            }
            if let Some(last_receive) = endpoint_meta.last_receive {
                if Instant::now() - last_receive >= endpoint_meta.endpoint_timeout / 2 {
                    idle_unnotified.push(*endpoint);
                    continue;
                }
            }
            if let Some(last_send) = endpoint_meta.last_send {
                if Instant::now() - last_send >= endpoint_meta.endpoint_timeout / 2 {
                    idle_unnotified.push(*endpoint);
                }
            }
        }
        idle_unnotified
    }

    /// Indicate that an "endpoint idle" TransportNotice for this Endpoint has been sent.
    /// Returns whether an un-idle entry was found and marked as idle.
    pub fn mark_endpoint_as_idle_notified(&mut self, endpoint: Endpoint) -> bool {
        if let Some(endpoint_meta) = self.endpoint_meta.get_mut(&endpoint) {
            // Return false if already marked as idle
            if endpoint_meta.notified_of_idle {
                return false;
            }
            endpoint_meta.notified_of_idle = true;
            true
        } else {
            false
        }
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

    /// Requested by the Filter layer to remove a packet from the transmit queue. This is necessary
    /// to prevent the packet from being resent. It should be performed when the packet has been
    /// acknowledged by the other end (only the Filter layer knows when this happens).
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
            for PacketContainer { packet, info, tid } in container {
                // Add the packet to the list of retriable packets if enough time has passed since the last transmission
                if info.transmit_interval != Duration::ZERO
                    && Instant::now().duration_since(info.last_transmit) > info.transmit_interval
                {
                    info.last_transmit = Instant::now();
                    info.retry_count += 1;
                    retry_qualified.push((&*packet, *endpoint));
                }

                if info.retry_count >= TRANSPORT_RETRY_COUNT_LOG_THRESHOLD && !info.retry_logged {
                    info.retry_logged = true;
                    warn!(
                        "[T] Retry logging threshold exceeded for endpoint {:?}, tid {}",
                        endpoint, tid
                    );
                }
            }
        }

        retry_qualified
    }
}
