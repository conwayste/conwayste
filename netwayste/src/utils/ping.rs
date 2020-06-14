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

use std::collections::VecDeque;
use std::time::Instant;

const PING_FILTER_DEPTH: usize = 12;

pub struct PingFilter {
    pub average_latency_ms: u128,
    running_sum: u128,
    history: VecDeque<u128>,
    start_timestamp: Instant,
    in_progress: bool,
}

impl PingFilter {
    pub fn new() -> PingFilter {
        PingFilter {
            average_latency_ms: 0,
            running_sum: 0,
            history: VecDeque::with_capacity(PING_FILTER_DEPTH),
            start_timestamp: Instant::now(),
            in_progress: false,
        }
    }

    pub fn reset(&mut self) {
        let Self {
            ref mut average_latency_ms,
            ref mut running_sum,
            ref mut history,
            ref mut start_timestamp,
            ref mut in_progress,
        } = *self;

        *average_latency_ms = 0;
        *running_sum = 0;
        history.clear();
        *start_timestamp = Instant::now();
        *in_progress = false;
    }

    pub fn start(&mut self) {
        self.start_timestamp = Instant::now();
        self.in_progress = true;
    }

    pub fn update(&mut self) {
        if !self.in_progress {
            panic!("start() was not called so a duration cannot be computed");
        }

        let latency = Instant::now().duration_since(self.start_timestamp);
        let latency_ms = latency.as_millis();

        self.running_sum += latency_ms;
        self.history.push_back(latency_ms);

        // Wait for the filter to be populated
        if self.history.len() > PING_FILTER_DEPTH {
            // unwraps safe b/c of length check
            let oldest = self.history.pop_front().unwrap();
            self.running_sum -= oldest;
            self.average_latency_ms = (self.running_sum as f64 / PING_FILTER_DEPTH as f64) as u128;
            println!("Client-side Ping: {}", self.average_latency_ms); // PR_GATE
        }

        self.in_progress = false;
    }

    #[cfg(test)]
    fn set_start_time(&mut self, ms_in_past: u64) {
        self.in_progress = true;
        let opt_past_timestamp = Instant::now().checked_sub(Duration::from_millis(ms_in_past));
        if let Some(past_timestamp) = opt_past_timestamp {
            self.start_timestamp = past_timestamp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_filter_under_filled_does_not_set_latency() {
        let mut pf = PingFilter::new();

        (0..PING_FILTER_DEPTH).into_iter().for_each(|_| {
            pf.set_start_time(500);
            pf.update();
        });

        assert_eq!(pf.average_latency_ms, 0);
    }

    #[test]
    fn test_ping_filter_filled_sets_latency() {
        let mut pf = PingFilter::new();

        (0..=PING_FILTER_DEPTH).into_iter().for_each(|_| {
            pf.set_start_time(500);
            pf.update();
        });

        assert_eq!(pf.average_latency_ms, 500);

        // Perform an additional 12 for shiggles
        (0..=PING_FILTER_DEPTH).into_iter().for_each(|_| {
            pf.set_start_time(500);
            pf.update();
        });

        assert_eq!(pf.average_latency_ms, 500);
    }

    #[test]
    fn test_ping_filter_filled_sets_latency_with_varying_pings() {
        let mut pf = PingFilter::new();

        (0..=PING_FILTER_DEPTH*100).step_by(100).into_iter().for_each(|i| {
            pf.set_start_time(i as u64);
            pf.update();
        });

        assert_eq!(pf.average_latency_ms, 650);
    }

    #[test]
    #[should_panic(expected = "start() was not called so a duration cannot be computed")]
    fn test_ping_filter_update_called_before_start() {
        let mut pf = PingFilter::new();
        pf.update();
    }
}
