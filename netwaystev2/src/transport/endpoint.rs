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

pub(in crate::transport) struct EndpointData<T> {
    endpoint_meta: HashMap<Endpoint, EndpointMeta>,
    receive:       HashMap<Endpoint, VecDeque<T>>,
    transmit:      HashMap<Endpoint, VecDeque<(usize, T)>>,
    transmit_meta: HashMap<Endpoint, VecDeque<(usize, TransmitMeta)>>,
}

impl<T> EndpointData<T> {
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

    pub fn push_receive_queue(&mut self, endpoint: Endpoint, item: T) -> Result<()> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Endpoint not found in Receive Queue during Insert: {:?}",
                    endpoint
                ));
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(item);
            }
        }

        match self.endpoint_meta.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Endpoint not found in Receive Queue during Insert: {:?}",
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
        item: T,
        packet_timeout: Duration,
        max_retries: usize,
    ) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => {
                return Err(anyhow!(
                    "Endpoint Transport Queue not found during Insert: {:?}",
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
                    "Endpoint Transmit Metadata not found during Insert: {:?}",
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

    pub fn drain_receive_queue(&mut self, endpoint: Endpoint) -> Result<Vec<T>> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Endpoint Receieve Queue not found during Remove: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().drain(..).collect()),
        }
    }

    pub fn pop_transmit_queue(&mut self, endpoint: Endpoint) -> Result<Option<(usize, T)>> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Endpoint Transmit Queue not found during Remove: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().pop_front()),
        }
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

    // Splits the packet transmission data group into those that need retries and those that have exhausted all retries
    pub fn bisect_retries(&mut self) -> (Vec<(&T, Endpoint)>, Vec<(usize, Endpoint)>) {
        let mut retry_qualified: Vec<(usize, Endpoint)> = vec![];
        let mut exhausted: Vec<(usize, Endpoint)> = vec![];

        for (endpoint, t_metadata) in &mut self.transmit_meta {
            let (mut has_retries, retries_exhausted): (Vec<(usize, TransmitMeta)>, Vec<(usize, TransmitMeta)>) =
                t_metadata
                    .iter()
                    .partition(|(_tid, metadata)| (metadata.retry_count < metadata.max_retries));

            retry_qualified.extend(has_retries.iter_mut().filter_map(|(tid, metadata)| {
                if Instant::now() - metadata.last_transmit > metadata.packet_timeout {
                    metadata.retry_count += 1;
                    metadata.last_transmit = Instant::now();
                    Some((*tid, *endpoint))
                } else {
                    None
                }
            }));

            exhausted.extend(retries_exhausted.iter().map(|(tid, _metadata)| (*tid, *endpoint)));
        }

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
