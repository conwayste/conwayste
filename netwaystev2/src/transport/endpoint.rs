use super::interface::TransportQueueKind;
use crate::common::Endpoint;
use anyhow::{anyhow, Result};

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

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

/// The data for an endpoint, where P is the type of the packet.
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

    pub fn drain_receive_queue(&mut self, endpoint: Endpoint) -> Result<Vec<P>> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Receieve Queue drain failed. Endpoint not found: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().drain(..).collect()),
        }
    }

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

    pub fn queue_count(&mut self, endpoint: Endpoint, kind: TransportQueueKind) -> Option<usize> {
        // XXX handle when endpoint not found
        match kind {
            TransportQueueKind::Transmit => self.transmit.get(&endpoint).map(|queue| queue.len()),
            TransportQueueKind::Receive => self.receive.get(&endpoint).map(|queue| queue.len()),
        }
    }

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

    pub fn drop_packet(&mut self, endpoint: Endpoint, tid: usize) -> Result<()> {
        let mut queue_index = None;
        if let Some(tx_queue) = self.transmit.get(&endpoint) {
            queue_index = tx_queue
                .iter()
                .position(|PacketContainer { tid: drop_tid, .. }| *drop_tid == tid);
        }

        if let Some(index) = queue_index {
            self.transmit.get_mut(&endpoint).unwrap().remove(index).map_or(
                Err(anyhow!(
                    "Could not remove packet from TX queue. {:?} tid: {} queue_index: {}",
                    endpoint,
                    tid,
                    index
                )),
                |_| Ok(()),
            )?;

            return Ok(());
        }

        return Err(anyhow!("Endpoint not found during packet drop: {:?}", endpoint));
    }

    /// Gather a list of all packets that can be retried
    pub fn get_retriable(&mut self) -> Vec<(&P, Endpoint)> {
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

    pub fn get_timed_out(&mut self) -> Vec<(usize, Endpoint)> {
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
