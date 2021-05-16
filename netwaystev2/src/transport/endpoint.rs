use super::interface::TransportQueueKind;
use crate::common::Endpoint;
use anyhow::{anyhow, Result};

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Transport layers uses this to track packet-specific retries and timeouts
#[derive(Clone)]
struct PacketInfo {
    packet_timeout: Duration,
    last_transmit:  Instant,
    max_retries:    usize,
    retry_count:    usize,
}

impl PacketInfo {
    pub fn new(packet_timeout: Duration, max_retries: usize) -> Self {
        PacketInfo {
            packet_timeout,
            last_transmit: Instant::now(),
            max_retries,
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
    tid:    usize,
    packet: P,
    info:   PacketInfo,
}

impl<P> PacketContainer<P> {
    pub fn new(tid: usize, packet: P, info: PacketInfo) -> Self {
        PacketContainer { tid, packet, info }
    }
}

/// The data for an endpoint, where P is the generic type of the thing to send (Packet).
pub(in crate::transport) struct EndpointData<P> {
    endpoint_meta: HashMap<Endpoint, EndpointMeta>,
    receive:       HashMap<Endpoint, VecDeque<P>>,
    transmit:      HashMap<Endpoint, VecDeque<PacketContainer<P>>>,
}

impl<P> EndpointData<P> {
    pub fn new() -> Self {
        EndpointData {
            endpoint_meta: HashMap::new(),
            receive:       HashMap::new(),
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
            Entry::Occupied(entry) => return Err(anyhow!("Endpoint {:?} exists in Transmit Queue", entry.key()).into()),
        }

        match self.receive.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::new());
            }
            Entry::Occupied(entry) => return Err(anyhow!("Endpoint {:?} exists in Receive Queue", entry.key()).into()),
        }

        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(EndpointMeta::new(timeout));
            }
            Entry::Occupied(entry) => return Err(anyhow!("Endpoint {:?} exists Transmission Info", entry.key()).into()),
        }

        Ok(())
    }

    /// Enqueues data packets `item` to the received queue for the endpoint.
    /// Will report an error if the endpoint does not exist.
    pub fn push_receive_queue(&mut self, endpoint: Endpoint, item: P) -> Result<()> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!("Receive Queue push failed. Endpoint not found: {:?}", endpoint));
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(item);
            }
        }

        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Receive Queue last receive timestamp update failed. Endpoint not found: {:?}",
                    endpoint
                ));
            }
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
        tid: usize,
        item: P,
        packet_timeout: Duration,
        max_retries: usize,
    ) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Transport Queue push failed. Endpoint not found: {:?}",
                    endpoint
                ))
            }
            Entry::Occupied(mut entry) => entry.get_mut().push_back(PacketContainer::new(
                tid,
                item,
                PacketInfo::new(packet_timeout, max_retries),
            )),
        }

        Ok(())
    }

    /// Pops all received data packets from the receive queue for the endpoint.
    /// Will report an error if the endpoint does not exist.
    pub fn drain_receive_queue(&mut self, endpoint: Endpoint) -> Result<Vec<P>> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Receieve Queue drain failed. Endpoint not found: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().drain(..).collect()),
        }
    }

    /// Drops all data packets in a queue for the endpoint.
    /// Will report an error if the endpoint does not exist.
    pub fn clear_queue(&mut self, endpoint: Endpoint, kind: TransportQueueKind) -> Result<()> {
        match kind {
            TransportQueueKind::Transmit => {
                if let Some(tx_queue) = self.transmit.get_mut(&endpoint) {
                    tx_queue.clear()
                } else {
                    return Err(anyhow!(
                        "Transmit Queue clear failed. Endpoint not found: {:?}",
                        endpoint
                    ));
                }
            }
            TransportQueueKind::Receive => {
                if let Some(rx_queue) = self.receive.get_mut(&endpoint) {
                    rx_queue.clear()
                } else {
                    return Err(anyhow!(
                        "Receive Queue clear failed. Endpoint not found: {:?}",
                        endpoint
                    ));
                }
            }
        }
        Ok(())
    }

    /// Requested by the Filter layer to probe the active length of the queue-kind.
    /// Will report an error if the endpoint does not exist.
    pub fn queue_count(&mut self, endpoint: Endpoint, kind: TransportQueueKind) -> Option<usize> {
        // XXX handle when endpoint not found
        match kind {
            TransportQueueKind::Transmit => self.transmit.get(&endpoint).map(|queue| queue.len()),
            TransportQueueKind::Receive => self.receive.get(&endpoint).map(|queue| queue.len()),
        }
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

        if let None = self.transmit.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }
        if let None = self.receive.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }

        if let None = self.endpoint_meta.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }

        if invalid_endpoint.len() != 0 {
            Err(anyhow!(
                "Endpoint not found during endpoint drop: {:?}",
                invalid_endpoint
            ))
        } else {
            Ok(())
        }
    }

    /// Requested by the Filter layer to remove a packet from the transmit queue. One use-case is if a packet is
    /// needs to be cancelled.
    /// Will report an error if the endpoint does not exist.
    /// Will report an error if the tid does not exist.
    /// Will report an error if the packet could not be removed.
    pub fn drop_packet(&mut self, endpoint: Endpoint, tid: usize) -> Result<()> {
        let queue_index;
        if let Some(tx_queue) = self.transmit.get(&endpoint) {
            queue_index = tx_queue
                .iter()
                .position(|PacketContainer { tid: drop_tid, .. }| *drop_tid == tid);
        } else {
            return Err(anyhow!("Endpoint not found during packet drop: {:?}", endpoint));
        }

        if let Some(index) = queue_index {
            self.transmit.get_mut(&endpoint).unwrap().remove(index).map_or(
                Err(anyhow!(
                    "Could not remove packet from transmit queue. {:?} tid: {} queue_index: {}",
                    endpoint,
                    tid,
                    index
                )),
                |_| Ok(()),
            )?;

            return Ok(());
        } else {
            return Err(anyhow!("tid {} not found for endpoint {:?}", tid, endpoint));
        }
    }

    /// Returns a list of packets that can be retried across all endpoints.
    pub fn retriable_packets(&mut self) -> Vec<(&P, Endpoint)> {
        let mut retry_qualified = vec![];

        for (endpoint, container) in &mut self.transmit {
            retry_qualified.extend(container.iter_mut().filter_map(|PacketContainer { packet, info, .. }| {
                if info.retry_count < info.max_retries {
                    if Instant::now() - info.last_transmit > info.packet_timeout {
                        info.last_transmit = Instant::now();
                        info.retry_count += 1;
                        Some((&*packet, *endpoint))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }));
        }

        retry_qualified
    }

    /// Returns a list of packets (via transmit id) that have timed-out across all endpoints.
    pub fn timed_out_packets(&mut self) -> Vec<(usize, Endpoint)> {
        let mut timed_out = vec![];
        for (endpoint, container) in &mut self.transmit {
            timed_out.extend(container.iter().filter_map(|PacketContainer { tid, info, .. }| {
                if info.retry_count >= info.max_retries {
                    Some((*tid, *endpoint))
                } else {
                    None
                }
            }));
        }
        timed_out
    }
}
