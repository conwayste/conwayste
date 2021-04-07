use super::interface::TransportQueueKind;
use crate::common::Endpoint;
use anyhow::{anyhow, Result};

use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::time::{Duration, Instant};

struct TransmissionInfo {
    last_receive: Option<Instant>,
    timeout:      Duration,
}

impl TransmissionInfo {
    pub fn new(timeout: Duration) -> Self {
        TransmissionInfo {
            last_receive: None,
            timeout,
        }
    }
}

pub(in crate::transport) struct EndpointData<T> {
    //received_packets: HashMap<Endpoint, VecDeque<Packet>>,
    receive:           HashMap<Endpoint, VecDeque<T>>,
    transmit:          HashMap<Endpoint, VecDeque<T>>,
    transmission_info: HashMap<Endpoint, TransmissionInfo>,
}

impl<T> EndpointData<T> {
    pub fn new() -> Self {
        EndpointData {
            receive:           HashMap::new(),
            transmit:          HashMap::new(),
            transmission_info: HashMap::new(),
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

        match self.transmission_info.entry(endpoint) {
            Entry::Vacant(entry) => {
                entry.insert(TransmissionInfo::new(timeout));
            }
            Entry::Occupied(entry) => return Err(anyhow!("Endpoint {:?} exists Transmission Info", entry.key()).into()),
        }

        Ok(())
    }

    pub fn insert_receivequeue(&mut self, endpoint: Endpoint, item: T) -> Result<()> {
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

        match self.transmission_info.entry(endpoint) {
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

    pub fn insert_transmitqueue(&mut self, endpoint: Endpoint, item: T) -> Result<()> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Endpoint not found in Transmit Queue during Insert: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(item);
                Ok(())
            }
        }
    }

    pub fn remove_receivequeue(&mut self, endpoint: Endpoint) -> Result<Option<T>> {
        match self.receive.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Endpoint not found in Receieve Queue during Remove: {:?}",
                endpoint
            )),
            Entry::Occupied(mut entry) => Ok(entry.get_mut().pop_front()),
        }
    }

    pub fn remove_transmitqueue(&mut self, endpoint: Endpoint) -> Result<Option<T>> {
        match self.transmit.entry(endpoint) {
            Entry::Vacant(_) => Err(anyhow!(
                "Endpoint not found in Transmit Queue during Remove: {:?}",
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

    pub fn timed_out_endpoints(&mut self) -> Result<Vec<Endpoint>> {
        let mut timedout = vec![];
        for (endpoint, trans_info) in &self.transmission_info {
            if let Some(last_receive) = trans_info.last_receive {
                if Instant::now() - last_receive >= trans_info.timeout {
                    timedout.push(*endpoint);
                }
            }
        }
        Ok(timedout)
    }

    pub fn drop_endpoint(&mut self, endpoint: Endpoint) -> Result<()> {
        let mut invalid_endpoint = std::collections::HashSet::new();

        if let None = self.transmit.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }
        if let None = self.receive.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }
        if let None = self.transmission_info.remove(&endpoint) {
            invalid_endpoint.insert(endpoint);
        }

        if invalid_endpoint.len() != 0 {
            Err(anyhow!("Endpoint not found during drop: {:?}", invalid_endpoint))
        } else {
            Ok(())
        }
    }
}
