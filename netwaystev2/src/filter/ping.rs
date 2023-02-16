/*
 * Copyright (C) 2020 The Conwayste Developers
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::cmp::Eq;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

use rand::random;
use serde::{Deserialize, Serialize};

/// This indicates the number of samples needed by the latency filter to calculate a statistically
/// meaningful average.
pub(crate) const LATENCY_FILTER_DEPTH: usize = 4;
const MAX_INFLIGHT_PINGS: usize = 20; // Will forget identifiers for older pings

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct PingPong {
    pub nonce: u64,
}

impl PingPong {
    pub fn ping() -> PingPong {
        PingPong { nonce: random::<u64>() }
    }

    pub fn pong(nonce: u64) -> PingPong {
        PingPong { nonce }
    }
}

/// A moving-average filter used to level out the latencies calculated from network request/response times.
/// The generic parameter `N` is the type of the identifier used to reference concurrent in-flight
/// pings (in practice, this will be either a PingPong or a u64).
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LatencyFilter<N: Hash + Eq + Clone> {
    average_latency_ms: Option<u64>,
    running_sum:        u64,
    history:            VecDeque<u64>,
    start_timestamps:   HashMap<N, Instant>,
    recent_pings:       VecDeque<N>,
}

impl<N: Hash + Eq + Clone> LatencyFilter<N> {
    pub fn new() -> LatencyFilter<N> {
        LatencyFilter {
            average_latency_ms: None,
            running_sum:        0,
            history:            VecDeque::with_capacity(LATENCY_FILTER_DEPTH),
            start_timestamps:   HashMap::new(),
            recent_pings:       VecDeque::new(),
        }
    }

    /// Call this when you send a PingPong
    pub fn start(&mut self, id: N) {
        self.push_recent_pings(id.clone()); // Will erase older pings if too many
        self.start_timestamps.insert(id, Instant::now());
    }

    fn push_recent_pings(&mut self, id: N) {
        if self.start_timestamps.contains_key(&id) {
            return;
        }
        self.recent_pings.push_back(id);
        while self.recent_pings.len() > MAX_INFLIGHT_PINGS {
            let oldest_id = self.recent_pings.pop_front().unwrap(); // unwrap OK
            self.start_timestamps.remove(&oldest_id);
        }
    }

    pub fn get_millis(&self) -> Option<u64> {
        self.average_latency_ms
    }

    #[allow(unused)]
    pub fn get(&self) -> Option<Duration> {
        self.get_millis().map(|ms| Duration::from_millis(ms))
    }

    /// Call this when you receive a response to your PingPing. Once it has been called
    /// `LATENCY_FILTER_DEPTH` times, the `average_latency_ms` field will contain the one-way (1/2
    /// RTT) ping time in milliseconds.
    pub fn update(&mut self, id: N) {
        let start = self.start_timestamps.get(&id);
        if start.is_none() {
            error!(
                "[F] The LatencyFilter's start() was not called for this identifier so a duration cannot be computed."
            );
            return;
        }
        let start = start.unwrap(); // unwrap OK because of above check

        let latency = Instant::now().duration_since(*start);
        let latency_ms = latency.as_millis() as u64;

        self.running_sum += latency_ms;
        self.history.push_back(latency_ms);

        while self.history.len() > LATENCY_FILTER_DEPTH {
            // unwraps safe b/c of length check
            let oldest = self.history.pop_front().unwrap();
            self.running_sum -= oldest;
        }

        // Wait for the filter to be populated
        if self.history.len() == LATENCY_FILTER_DEPTH {
            // average round-trip time
            let average_rtt_ms = (self.running_sum as f64 / LATENCY_FILTER_DEPTH as f64) as u64;
            self.average_latency_ms = Some(average_rtt_ms / 2);
        }
    }

    #[cfg(test)]
    fn set_start_time(&mut self, id: N, ms_in_past: u64) {
        let opt_past_timestamp = Instant::now().checked_sub(Duration::from_millis(ms_in_past));
        if let Some(past_timestamp) = opt_past_timestamp {
            if let Some(ts) = self.start_timestamps.get_mut(&id) {
                *ts = past_timestamp;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_filter_under_filled_does_not_set_latency() {
        let mut pf = LatencyFilter::new();

        (0..(LATENCY_FILTER_DEPTH - 1)).into_iter().for_each(|id| {
            pf.start(id);
            pf.set_start_time(id, 500);
            pf.update(id);
        });

        assert_eq!(pf.average_latency_ms, None);
    }

    #[test]
    fn test_latency_filter_filled_sets_latency() {
        let mut pf = LatencyFilter::new();

        (0..LATENCY_FILTER_DEPTH).into_iter().for_each(|id| {
            pf.start(id);
            pf.set_start_time(id, 500);
            pf.update(id);
        });

        assert_eq!(pf.average_latency_ms, Some(250));

        // Perform an additional 12 for shiggles
        (0..=LATENCY_FILTER_DEPTH).into_iter().for_each(|x| {
            let id = x + 100;
            pf.start(id);
            pf.set_start_time(id, 500);
            pf.update(id);
        });

        assert_eq!(pf.average_latency_ms, Some(250));
    }

    #[test]
    #[ignore]
    fn test_latency_filter_filled_sets_latency_with_varying_pings() {
        let mut pf = LatencyFilter::new();

        let mut id = 123;
        (0..LATENCY_FILTER_DEPTH * 100).step_by(100).into_iter().for_each(|i| {
            pf.start(id);
            pf.set_start_time(id, i as u64);
            pf.update(id);
            id += 1;
        });

        assert_eq!(pf.average_latency_ms, Some(325));
    }
}
