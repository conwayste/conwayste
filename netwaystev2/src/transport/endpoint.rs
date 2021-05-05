use super::interface::TransportQueueKind;
use crate::common::Endpoint;
use anyhow::{anyhow, Result};

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Copy, Clone)]
struct TransmitMeta {
    packet_timeout: Duration,
    last_transmit:  Instant,
    max_retries:    usize,
    retry_count:    usize,
}

impl TransmitMeta {
    pub fn new(packet_timeout: Duration, max_retries: usize) -> Self {
        TransmitMeta {
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

/// The data for an endpoint, where P is the type of the packet.
pub(in crate::transport) struct EndpointData<P> {
    endpoint_meta: HashMap<Endpoint, EndpointMeta>,
    receive:       HashMap<Endpoint, VecDeque<P>>,
    transmit:      HashMap<Endpoint, VecDeque<(usize, P)>>,
    transmit_meta: HashMap<Endpoint, VecDeque<(usize, TransmitMeta)>>,
}

impl<P> EndpointData<P> {
    pub fn new() -> Self {
        EndpointData {
            endpoint_meta: HashMap::new(),
            receive:       HashMap::new(),
            transmit:      HashMap::new(),
            transmit_meta: HashMap::new(),
        }
    }

    pub fn new_endpoint(&mut self, endpoint: Endpoint, timeout: Duration) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::new());
            }
            Entry::Occupied(entry) => return Err(anyhow!("Endpoint {:?} exists in Transmit Queue", entry.key()).into()),
        }

        match self.transmit_meta.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::new());
            }
            Entry::Occupied(entry) => {
                return Err(anyhow!("Endpoint {:?} exists in Transmit Meta Queue", entry.key()).into())
            }
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
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back((tid, item));
            }
        }

        match self.transmit_meta.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Transmit Metadata Queue push failed. Endpoint not found: {:?}",
                    endpoint
                ))
            }
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .push_back((tid, TransmitMeta::new(packet_timeout, max_retries)));
            }
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

    pub fn pop_transmit_queue(&mut self, endpoint: Endpoint) -> Result<Option<(usize, P)>> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!("Transmit Queue pop failed. Endpoint not found: {:?}", endpoint)),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().pop_front()),
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

                if let Some(tx_meta_queue) = self.transmit_meta.get_mut(&endpoint) {
                    tx_meta_queue.clear()
                } else {
                    return Err(anyhow!(
                        "Transmit Meta Queue clear failed. Endpoint not found: {:?}",
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
        if let None = self.transmit_meta.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }

        if invalid_endpoint.len() != 0 {
            Err(anyhow!("Endpoint not found during drop: {:?}", invalid_endpoint))
        } else {
            Ok(())
        }
    }

    /// Splits the packet transmission data group into those that need retries and those that have exhausted all retries
    pub fn separate_into_retriable_and_timed_out(&mut self) -> (Vec<(&P, Endpoint)>, Vec<(usize, Endpoint)>) {
        let mut retry_qualified: Vec<(usize, Endpoint)> = vec![];
        let mut exhausted: Vec<(usize, Endpoint)> = vec![];

        for (endpoint, t_metadata) in &mut self.transmit_meta {
            // Split packets into those that can be retried and those that ran out
            let (mut has_retries, retries_exhausted): (Vec<(usize, TransmitMeta)>, Vec<(usize, TransmitMeta)>) =
                t_metadata
                    .iter()
                    .partition(|(_tid, metadata)| (metadata.retry_count < metadata.max_retries));

            // Find retriable packets that have timed-out
            retry_qualified.extend(has_retries.iter_mut().filter_map(|(tid, metadata)| {
                if Instant::now() - metadata.last_transmit > metadata.packet_timeout {
                    Some((*tid, *endpoint))
                } else {
                    None
                }
            }));

            // Advance the retry
            for (retry_tid, _) in retry_qualified.iter() {
                for (update_tid, update_meta) in t_metadata.iter_mut() {
                    if retry_tid == update_tid {
                        update_meta.last_transmit = Instant::now();
                        update_meta.retry_count += 1;
                    }
                }
            }

            exhausted.extend(retries_exhausted.iter().map(|(tid, _metadata)| (*tid, *endpoint)));
        }

        // Map packets that can be retried into their data and destination
        let mut retry_datagrams = vec![];
        for (retry_tid, endpoint) in retry_qualified {
            if let Some(data_pairs) = self.transmit.get(&endpoint) {
                retry_datagrams.extend(data_pairs.into_iter().filter_map(|(tid, data)| {
                    if retry_tid == *tid {
                        Some((data, endpoint))
                    } else {
                        None
                    }
                }));
            }
        }

        (retry_datagrams, exhausted)
    }
}
