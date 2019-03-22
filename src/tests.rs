/*  Copyright 2019 the Conwayste Developers.
 *
 *  This file is part of netwayste.
 *
 *  netwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  netwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with netwayste.  If not, see <http://www.gnu.org/licenses/>. */

use std::{thread, time::{Instant, Duration}};
use std::net::SocketAddr;
use crate::net::*;
use crate::futures::sync::mpsc;

mod netwayste_net_tests {
    use super::*;

    fn fake_socket_addr() -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5678)
    }


    // `discord_older_packets()` tests are disabled  until after the necessity of the function is re-evaluated
    #[test]
    #[ignore]
    fn test_discard_older_packets_empty_queue() {
        let mut nm = NetworkManager::new();

        nm.tx_packets.discard_older_items();
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), 0);
        assert_eq!(nm.rx_packets.len(), 0);
    }

    #[test]
    #[ignore]
    fn test_discard_older_packets_under_limit_keeps_all_messages() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.push_back(pkt.clone());
        nm.tx_packets.push_back(pkt.clone());
        nm.tx_packets.push_back(pkt.clone());

        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), 3);

        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());
        nm.rx_packets.push_back(pkt.clone());

        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), 3);
    }

    #[test]
    #[ignore]
    fn test_discard_older_packets_equal_to_limit() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH);
        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH-1);

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
    }

    #[test]
    #[ignore]
    fn test_discard_older_packets_exceeds_limit_retains_max() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH+10 {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH+10);
        nm.tx_packets.discard_older_items();
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH-1);

        for _ in 0..NETWORK_QUEUE_LENGTH+5 {
            nm.rx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH+5);
        nm.rx_packets.discard_older_items();
        assert_eq!(nm.rx_packets.len(), NETWORK_QUEUE_LENGTH);
    }

    #[test]
    fn test_buffer_item_queue_is_empty() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 1);
    }

    #[test]
    fn test_buffer_item_sequence_number_reused() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = nm.tx_packets.queue.back().unwrap();
        if let Packet::Request { sequence: _, response_ack: _, cookie: _, action } = pkt {
            assert_eq!(*action, RequestAction::None);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequencing() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 2);
    }

    #[test]
    fn test_buffer_item_newer_packet_has_smaller_sequence_number() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 1,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        nm.tx_packets.buffer_item(pkt);
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::LeaveRoom
        };
        nm.tx_packets.buffer_item(pkt);
        assert_eq!(nm.tx_packets.len(), 2);

        let pkt = nm.tx_packets.queue.back().unwrap();
        if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
            assert_eq!(*sequence, 1);
        }
    }


    // `buffer_item()` test with an enforced hard limit size is disabled until performance is re-examined
    #[test]
    #[ignore]
    fn test_buffer_item_max_queue_limit_maintained() {
        let mut nm = NetworkManager::new();
        for index in 0..NETWORK_QUEUE_LENGTH+5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.tx_packets.buffer_item(pkt);
        }

        let mut iter =  nm.tx_packets.queue.iter();
        for index in 5..NETWORK_QUEUE_LENGTH+5 {
            let pkt = iter.next().unwrap();
            if let Packet::Request { sequence, response_ack: _, cookie: _, action:_ } = pkt {
                assert_eq!(*sequence, index as u64);
            }
        }
    }

    #[test]
    fn test_buffer_item_basic_contiguous_ascending() {
        let mut nm = NetworkManager::new();
        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_contiguous_descending() {
        let mut nm = NetworkManager::new();
        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequential_gap_ascending() {
        let mut nm = NetworkManager::new();
        // TODO Replace with (x,y).step_by(z) once stable
        for index in [0,2,4,6,8,10].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for &index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_sequential_gap_descending() {
        let mut nm = NetworkManager::new();
        for index in [0,2,4,6,8,10].iter().rev() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0,2,4,6,8,10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_basic_random() {
        let mut nm = NetworkManager::new();
        for index in [5, 2, 9, 1, 0, 8, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 5, 6, 8, 9].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_butterfly_pattern() {
        let mut nm = NetworkManager::new();
        // This one is fun because it tests the internal edges of (front_slice and back_slice)
        for index in [0, 10, 1, 9, 2, 8, 3, 7, 4, 6, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_repetition() {
        let mut nm = NetworkManager::new();
        for index in [0, 0, 0, 0, 1, 2, 2, 2, 5].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        for index in [0, 1, 2, 5].iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_advanced_sequential_then_pseudorandom_then_sequential() {
        let mut nm = NetworkManager::new();

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in 13..20 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_advanced_reverse_sequential_then_random_then_reverse_sequential() {
        let mut nm = NetworkManager::new();

        for index in (0..5).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [10, 7, 11, 9, 12, 8, 99, 6].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in (13..20).rev() {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (0..20).collect::<Vec<usize>>();
        range.extend([99].iter().cloned()); // Add in 99
        range.remove(5); // But remove 5 since it was never included
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number() as usize);
        }
    }

    #[test]
    fn test_buffer_item_basic_wrapping_case() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let start = u64_max - 5;

        for index in start..(start+5) {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_basic_wrapping_case_then_out_of_order() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let start = u64_max - 5;

        for index in start..(start+5) {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        {
            let pkt = Packet::Request {
                sequence: u64_max,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        for index in [5, 0, 4, 1, 3, 2].iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = (start..u64_max).collect::<Vec<u64>>();
        range.extend([u64_max, 0, 1, 2, 3, 4, 5].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_everything_out_of_order() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [ max_minus_4,
                            two,
                            max_minus_1,
                            max_minus_5,
                            u64_max,
                            three,
                            max_minus_2,
                            zero,
                            max_minus_3,
                            one ];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned()); // Add in u64 max value plus others

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_max_sequence_number_arrives_after_a_wrap() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_1, max_minus_2, three, u64_max, two];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_2, max_minus_1, u64_max, two, three].iter().cloned()); // Add in u64 max value plus others
        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_oldest_sequence_number_arrived_last() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_1, max_minus_2, three, u64_max, two, one, zero, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrap_occurs_with_two_item_queue() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        // Forward wrap occurs non-contiguously (aka [254, 0, ...] for bytes)
        let input_order = [max_minus_1, zero, three, u64_max, max_minus_2, one, two, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrap_occurs_with_two_item_queue_in_reverse() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        // Wrap takes place in reverse order ( aka [0, 254, ...] for bytes)
        let input_order = [zero, max_minus_1, three, u64_max, max_minus_2, one, two, max_minus_3];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_max_arrives_first() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [u64_max, max_minus_4, two, max_minus_1, max_minus_5, three, max_minus_2, zero, max_minus_3, one];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three].iter().cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_sequence_number_descending() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [three, two, one, zero, u64_max, max_minus_1, max_minus_2, max_minus_3, max_minus_4, max_minus_5];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned());

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_buffer_item_advanced_wrapping_case_sequence_number_alternating() {
        let mut nm = NetworkManager::new();
        let u64_max = <u64>::max_value();
        let max_minus_5 = u64_max - 5;
        let max_minus_4 = u64_max - 4;
        let max_minus_3 = u64_max - 3;
        let max_minus_2 = u64_max - 2;
        let max_minus_1 = u64_max - 1;
        let zero = 0;
        let one = 1;
        let two = 2;
        let three = 3;

        let input_order = [max_minus_5, three, max_minus_4, two, max_minus_3, one, max_minus_2, zero, max_minus_1, u64_max];

        for index in input_order.iter() {
            let pkt = Packet::Request {
                sequence: *index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let mut iter = nm.rx_packets.queue.iter();
        let mut range = vec![];
        range.extend([max_minus_5, max_minus_4, max_minus_3, max_minus_2, max_minus_1, u64_max, zero, one, two, three]
                .iter()
                .cloned()); // Add in u64 max value plus others

        for index in range.iter() {
            let pkt = iter.next().unwrap();
            assert_eq!(*index, pkt.sequence_number());
        }
    }

    #[test]
    fn test_reinitialize_all_queues_cleared() {
        let mut nm = NetworkManager::new();
        let pkt = Packet::Request {
            sequence: 0,
            response_ack: None,
            cookie: None,
            action: RequestAction::None
        };

        for _ in 0..NETWORK_QUEUE_LENGTH {
            nm.tx_packets.push_back(pkt.clone());
        }
        assert_eq!(nm.tx_packets.len(), NETWORK_QUEUE_LENGTH);

        let _chat_msg = BroadcastChatMessage::new(0, "chatchat".to_owned(), "chatchat".to_owned());
    }

    #[test]
    fn test_get_contiguous_packets_count() {
        let mut nm = NetworkManager::new();
        for index in 0..5 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }
        for index in 8..10 {
            let pkt = Packet::Request {
                sequence: index as u64,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };
            nm.rx_packets.buffer_item(pkt);
        }

        let count = nm.rx_packets.get_contiguous_packets_count(0);
        assert_eq!(count, 5);
        let mut iter = nm.rx_packets.as_queue_type().iter();
        for index in 0..5 {
            let pkt = iter.next().unwrap();
            assert_eq!(index, pkt.sequence_number() as usize);
            // Verify that the packet is not dequeued
            assert_eq!(index, nm.rx_packets.as_queue_type().get(index).unwrap().sequence_number() as usize);
        }
    }

    #[test]
    fn test_get_retransmit_indices() {
        let mut nm = NetworkManager::new();
        for i in 0..5 {
            let pkt = Packet::Request {
                sequence: i,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };

            nm.tx_packets.buffer_item(pkt.clone());

            if i < 3 {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.back_mut().unwrap();
                attempt.time = Instant::now() - Duration::from_secs(i+1);
            }
        }
        assert_eq!(nm.tx_packets.get_retransmit_indices().len(), 3);
        thread::sleep(Duration::from_millis(2000));
        assert_eq!(nm.tx_packets.get_retransmit_indices().len(), 5);
    }

    #[test]
    fn test_retransmit_expired_tx_packets_no_expirations() {
        let mut nm = NetworkManager::new();

        for i in 0..5 {
            let pkt = Packet::Request {
                sequence: i,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };

            nm.tx_packets.buffer_item(pkt.clone());
        }

        let indices = nm.tx_packets.get_retransmit_indices();

        let (udp_tx, _) = mpsc::unbounded();
        let addr = fake_socket_addr();
        nm.retransmit_expired_tx_packets(&udp_tx, addr, None, &indices);

        for i in 0..5 {
            assert_eq!(nm.tx_packets.attempts.get(i).unwrap().retries, 0);
        }
    }

    #[test]
    fn test_retransmit_expired_tx_packets_basic_retries() {
        let mut nm = NetworkManager::new();

        for i in 0..5 {
            let pkt = Packet::Request {
                sequence: i,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };

            nm.tx_packets.buffer_item(pkt.clone());

           if i < 3 {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.back_mut().unwrap();
                attempt.time = Instant::now() - Duration::from_secs(i+1);
            }
        }

        let indices = nm.tx_packets.get_retransmit_indices();

        let (udp_tx, _) = mpsc::unbounded();
        let addr = fake_socket_addr();
        nm.retransmit_expired_tx_packets(&udp_tx, addr, None, &indices);

        for i in 0..3 {
            assert_eq!(nm.tx_packets.attempts.get(i).unwrap().retries, 1);
        }
        for i in 3..5 {
            assert_eq!(nm.tx_packets.attempts.get(i).unwrap().retries, 0);
        }
    }

    #[test]
    fn test_retransmit_expired_tx_packets_aggressive_retries() {
        let mut nm = NetworkManager::new();

        for i in 0..5 {
            let pkt = Packet::Request {
                sequence: i,
                response_ack: None,
                cookie: None,
                action: RequestAction::None
            };

            nm.tx_packets.buffer_item(pkt.clone());

           if i < 3 {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.back_mut().unwrap();
                attempt.time = Instant::now() - Duration::from_secs(i+1);
            }
        }

        // After 2 attempts, aggressive mode should kick in
        for _ in 0..5 {
            let indices = nm.tx_packets.get_retransmit_indices();

            println!("{:?}", indices);

            let (udp_tx, _) = mpsc::unbounded();
            let addr = fake_socket_addr();
            nm.retransmit_expired_tx_packets(&udp_tx, addr, None, &indices);

            for j in 0..indices.len() {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.get_mut(j).unwrap();
                attempt.time = Instant::now() - Duration::from_secs( 1u64);
            }
        }

        for i in 0..3 {
            // 5 + 2 + 2 + 3
            assert_eq!(nm.tx_packets.attempts.get(i).unwrap().retries, 11);
        }
        for i in 3..5 {
            assert_eq!(nm.tx_packets.attempts.get(i).unwrap().retries, 0);
        }

    }

}

mod netwayste_client_tests {

}

mod netwayste_server_tests {
    use super::*;
    use crate::proptest::strategy::*;
    use crate::server::*;

    fn fake_socket_addr() -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5678)
    }

    #[test]
    fn list_players_player_shows_up_in_player_list() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));

        let (player_id, player_name) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.name.clone())
        };
        // make the player join the room
        {
            server.join_room(player_id, String::from(room_name));
        }
        let resp_code: ResponseCode = server.list_players(player_id);
        match resp_code {
            ResponseCode::PlayerList(players) => {
                assert_eq!(players.len(), 1);
                assert_eq!(*players.first().unwrap(), player_name);
            }
            resp_code @ _ => panic!("Unexpected response code: {:?}", resp_code)
        }
    }

    #[test]
    fn has_chatted_player_did_not_chat_on_join() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));
        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());
            p.player_id
        };
        // make the player join the room
        {
            server.join_room(player_id, String::from(room_name));
        }
        let player = server.get_player(player_id);
        assert_eq!(player.has_chatted(), false);
    }

    #[test]
    fn get_confirmed_chat_seq_num_server_tracks_players_chat_updates() {
        let mut server = ServerState::new();
        let room_name = "some name";
        // make a new room
        server.create_new_room(None, String::from(room_name));

        let (player_id, player_cookie) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.cookie.clone())
        };
        // make the player join the room
        {
            server.join_room(player_id, String::from(room_name));
        }

        // A chat-less player now has something to to say
        server.decode_packet(fake_socket_addr(), Packet::UpdateReply {
            cookie: player_cookie.clone(),
            last_chat_seq: Some(1),
            last_game_update_seq: None,
            last_gen: None
        }).unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }

        // Older messages are ignored
        server.decode_packet(fake_socket_addr(), Packet::UpdateReply {
            cookie: player_cookie.clone(),
            last_chat_seq: Some(0),
            last_game_update_seq: None,
            last_gen: None
        }).unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }

        // So are absent messages
        server.decode_packet(fake_socket_addr(), Packet::UpdateReply {
            cookie: player_cookie,
            last_chat_seq: None,
            last_game_update_seq: None,
            last_gen: None
        }).unwrap();

        {
            let player = server.get_player(player_id);
            assert_eq!(player.get_confirmed_chat_seq_num(), Some(1));
        }
    }

    #[test]
    fn get_message_skip_count_player_acked_messages_are_not_included_in_skip_count() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let (player_id, _) = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            (p.player_id, p.cookie.clone())
        };
        // make the player join the room
        // Give it a single message
        {
            server.join_room(player_id, String::from(room_name));
            server.handle_chat_message(player_id, "ChatMessage".to_owned());
        }

        {
            let room: &Room = server.get_room(player_id).unwrap();
            // The check below does not affect any player acknowledgement as we are not
            // involving the player yet. This is a simple test to ensure that if a chat
            // message decoded from a would-be player was less than the latest chat message,
            // we handle it properly by not skipping any.
            assert_eq!(room.get_message_skip_count(0), 0);
        }

        let number_of_messages = 6;
        for _ in 1..number_of_messages {
            server.handle_chat_message(player_id, "ChatMessage".to_owned());
        }

        {
            //let player = server.get_player_mut(player_id);
            let player = server.get_player_mut(player_id);
            // player has not acknowledged any yet
            #[should_panic]
            assert_eq!(player.get_confirmed_chat_seq_num(), None);
        }

        // player acknowledged four of the six
        let acked_message_count = {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(4));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.get_message_skip_count(acked_message_count), acked_message_count);
        }

        // player acknowledged all six
        let acked_message_count = {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(6));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.get_message_skip_count(acked_message_count), acked_message_count);
        }
    }

    #[test]
    // Send fifteen messages, but only leave nine unacknowledged, while wrapping on the sequence number
    fn get_message_skip_count_player_acked_messages_are_not_included_in_skip_count_wrapped_case() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, String::from(room_name));
        }

        // Picking a value slightly less than max of u64
        let start_seq_num = u64::max_value() - 6;
        // First pass, add messages with sequence numbers through the max of u64
        for seq_num in start_seq_num..u64::max_value() {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(player_id, String::from("some name"), String::from("some msg"), seq_num));
        }
        // Second pass, from wrap-point, `0`, eight times
        for seq_num in 0..8 {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(player_id, String::from("some name"), String::from("some msg"), seq_num));
        }

        let acked_message_count = {
            // Ack up until 0xFFFFFFFFFFFFFFFD
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(start_seq_num + 4));

            player.get_confirmed_chat_seq_num().unwrap()
        };
        {
            let room: &Room = server.get_room(player_id).unwrap();
            // Fifteen total messages sent.
            // 2 unacked which are less than u64::max_value()
            // 8 unacked which are after the numerical wrap
            let unacked_count = 15 - (8 + 2);
            assert_eq!(room.get_message_skip_count(acked_message_count), unacked_count);
        }
    }

    #[test]
    fn collect_unacknowledged_messages_a_rooms_unacknowledged_chat_messages_are_collected_for_their_player() {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, String::from(room_name));
        }

        {
            // Room has no messages, None to send to player
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }

        {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(player_id, String::from("some name"), String::from("some msg"), 1));
        }
        {
            // Room has a message, player has yet to ack it
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages.is_some(), true);
            assert_eq!(messages.unwrap().len(), 1);
        }

        {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(1));
        }
        {
            // Room has a message, player acked, None
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }
    }

    #[test]
    fn collect_unacknowledged_messages_an_active_room_which_expired_all_messages_returns_none()
    {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, String::from(room_name));

        let player_id = {
            let p: &mut Player = server.add_new_player(String::from("some name"), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, String::from(room_name));
        }

        {
            // Add a message to the room and then age it so it will expire
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.add_message(ServerChatMessage::new(player_id, String::from("some name"), String::from("some msg"), 1));

            let message: &mut ServerChatMessage = room.messages.get_mut(0).unwrap();
            message.timestamp = Instant::now() - Duration::from_secs(MAX_AGE_CHAT_MESSAGES as u64);
        }
        {
            // Sanity check to ensure player gets the chat message if left unacknowledged
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages.is_some(), true);
            assert_eq!(messages.unwrap().len(), 1);
        }
        {
            let player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(1));
        }

        {
            // Server drains expired messages for the room
            server.expire_old_messages_in_all_rooms();
        }
        {
            // A room that has no messages, but has player(s) who have acknowledged past messages
            let room = server.get_room(player_id).unwrap();
            let player = server.get_player(player_id);
            let messages = server.collect_unacknowledged_messages(room, player);
            assert_eq!(messages, None);
        }
    }

    #[test]
    fn handle_chat_message_player_not_in_game()
    {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some name".to_owned(), fake_socket_addr());

            p.player_id
        };

        let response = server.handle_chat_message(player_id, "test msg".to_owned());
        assert_eq!(response, ResponseCode::BadRequest(Some(format!("Player {} has not joined a game.", player_id))));
    }

    #[test]
    fn handle_chat_message_player_in_game_one_message()
    {

        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_string(), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name.to_owned());
        }

        let response = server.handle_chat_message(player_id, "test msg".to_owned());
        assert_eq!(response, ResponseCode::OK);
        let room: &Room = server.get_room(player_id).unwrap();
        assert_eq!(room.messages.len(), 1);
        assert_eq!(room.latest_seq_num, 1);
        assert_eq!(room.get_newest_msg(), room.get_oldest_msg());
    }

    #[test]
    fn handle_chat_message_player_in_game_many_messages()
    {

        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name.to_owned());
        }

        let response = server.handle_chat_message(player_id, "test msg first".to_owned());
        assert_eq!(response, ResponseCode::OK);
        let response = server.handle_chat_message(player_id, "test msg second".to_owned());
        assert_eq!(response, ResponseCode::OK);

        let room: &Room = server.get_room(player_id).unwrap();
        assert_eq!(room.messages.len(), 2);
        assert_eq!(room.latest_seq_num, 2);
    }

    #[test]
    fn create_new_room_good_case()
    {
        {
            let mut server = ServerState::new();
            let room_name = "some name".to_owned();

            assert_eq!(server.create_new_room(None, room_name), ResponseCode::OK);
        }
        // Room name length is within bounds
        {
            let mut server = ServerState::new();
            let room_name = "0123456789ABCDEF".to_owned();

            assert_eq!(server.create_new_room(None, room_name), ResponseCode::OK);
        }
    }

    #[test]
    fn create_new_room_name_is_too_long()
    {
        let mut server = ServerState::new();
        let room_name = "0123456789ABCDEF_#".to_owned();

        assert_eq!(server.create_new_room(None, room_name), ResponseCode::BadRequest(Some("room name too long; max 16 characters".to_owned())));
    }

    #[test]
    fn create_new_room_name_taken()
    {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);
        assert_eq!(server.create_new_room(None, room_name), ResponseCode::BadRequest(Some("room name already in use".to_owned())));
    }

    #[test]
    fn create_new_room_player_already_in_room()
    {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        let other_room_name = "another room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name.to_owned());
        }

        assert_eq!( server.create_new_room(Some(player_id), other_room_name), ResponseCode::BadRequest(Some("cannot create room because in-game".to_owned())) );
    }

    #[test]
    fn create_new_room_join_room_good_case()
    {

        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(server.join_room(player_id, room_name.to_owned()), ResponseCode::JoinedRoom("some room".to_owned()));
    }

    #[test]
    fn join_room_player_already_in_room()
    {

        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(server.join_room(player_id, room_name.clone()), ResponseCode::JoinedRoom("some room".to_owned()));
        assert_eq!( server.join_room(player_id, room_name), ResponseCode::BadRequest(Some("cannot join game because in-game".to_owned())) );
    }

    #[test]
    fn join_room_room_does_not_exist()
    {

        let mut server = ServerState::new();

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        assert_eq!(server.join_room(player_id, "some room".to_owned()), ResponseCode::BadRequest(Some("no room named \"some room\"".to_owned())) );
    }

    #[test]
    fn leave_room_good_case()
    {
        let mut server = ServerState::new();
        let room_name = "some name";

        server.create_new_room(None, room_name.to_owned());

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };
        {
            server.join_room(player_id, room_name.to_owned());
        }

        assert_eq!( server.leave_room(player_id), ResponseCode::LeaveRoom );

    }

    #[test]
    fn leave_room_player_not_in_room()
    {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        let player_id = {
            let p: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());

            p.player_id
        };

        assert_eq!( server.leave_room(player_id), ResponseCode::BadRequest(Some("cannot leave game because in lobby".to_owned())) );
    }

    #[test]
    fn leave_room_unregistered_player_id()
    {
        let mut server = ServerState::new();
        let room_name = "some room".to_owned();
        let rand_player_id = PlayerID(0x2457); //RUST
        assert_eq!(server.create_new_room(None, room_name.clone()), ResponseCode::OK);

        assert_eq!( server.leave_room(rand_player_id), ResponseCode::BadRequest(Some("cannot leave game because in lobby".to_owned())) );
    }

    #[test]
    fn add_new_player_player_added_with_initial_sequence_number()
    {
        let mut server = ServerState::new();
        let name = "some player".to_owned();

        let p: &mut Player = server.add_new_player(name.clone(), fake_socket_addr());
        assert_eq!(p.name, name);
    }

    #[test]
    fn is_unique_player_name_yes_and_no_case()
    {
        let mut server = ServerState::new();
        let name = "some player".to_owned();
        assert_eq!(server.is_unique_player_name("some player"), true);

        {
            server.add_new_player(name.clone(), fake_socket_addr());
        }
        assert_eq!(server.is_unique_player_name("some player"), false);
    }

    #[test]
    fn expire_old_messages_in_all_rooms_room_is_empty()
    {
        let mut server = ServerState::new();
        let room_name = "some room";

        server.create_new_room(None, room_name.to_owned().clone());
        server.expire_old_messages_in_all_rooms();

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }


    #[test]
    fn expire_old_messages_in_all_rooms_one_room_good_case()
    {
        let mut server = ServerState::new();
        let room_name = "some room";

        server.create_new_room(None, room_name.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.to_owned());

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id, "It is free!".to_owned());
        server.handle_chat_message(player_id, "What's not to love?".to_owned());

        let message_count = {
            let room: &Room = server.get_room(player_id).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count, 4);

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms();

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 4);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_several_rooms_good_case()
    {
        let mut server = ServerState::new();
        let room_name = "some room";
        let room_name2 = "some room2";

        server.create_new_room(None, room_name.to_owned().clone());
        server.create_new_room(None, room_name2.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.to_owned());
        server.join_room(player_id2, room_name2.to_owned());

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id2, "It is free!".to_owned());
        server.handle_chat_message(player_id2, "What's not to love?".to_owned());

        let message_count = {
            let room: &Room = server.get_room(player_id).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count, 2);
        let message_count2 = {
            let room: &Room = server.get_room(player_id2).unwrap();
            room.messages.len()
        };
        assert_eq!(message_count2, 2);

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms();

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 2);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_one_room_old_messages_are_wiped()
    {
        let mut server = ServerState::new();
        let room_name = "some room";

        server.create_new_room(None, room_name.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.to_owned());

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id, "It is free!".to_owned());
        server.handle_chat_message(player_id, "What's not to love?".to_owned());

        let current_timestamp = Instant::now();
        let travel_to_the_past = current_timestamp - Duration::from_secs((MAX_AGE_CHAT_MESSAGES+1) as u64);
        for ref mut room in server.rooms.values_mut() {
            println!("Room: {:?}", room.name);
            for m in room.messages.iter_mut() {
                println!("{:?}, {:?},       {:?}", m.timestamp, travel_to_the_past, m.timestamp - travel_to_the_past);
                m.timestamp = travel_to_the_past;
            }
        }

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms();

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }

    #[test]
    fn expire_old_messages_in_all_rooms_several_rooms_old_messages_are_wiped()
    {
        let mut server = ServerState::new();
        let room_name = "some room";
        let room_name2 = "some room 2";

        server.create_new_room(None, room_name.to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        server.create_new_room(None, room_name2.to_owned().clone());
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.to_owned());
        server.join_room(player_id2, room_name.to_owned());

        server.handle_chat_message(player_id, "Conwayste is such a fun game".to_owned());
        server.handle_chat_message(player_id, "There are not loot boxes".to_owned());
        server.handle_chat_message(player_id2, "It is free!".to_owned());
        server.handle_chat_message(player_id2, "What's not to love?".to_owned());

        let current_timestamp = Instant::now();
        let travel_to_the_past = current_timestamp - Duration::from_secs((MAX_AGE_CHAT_MESSAGES+1) as u64);
        for ref mut room in server.rooms.values_mut() {
            println!("Room: {:?}", room.name);
            for m in room.messages.iter_mut() {
                println!("{:?}, {:?},       {:?}", m.timestamp, travel_to_the_past, m.timestamp - travel_to_the_past);
                m.timestamp = travel_to_the_past;
            }
        }

        // Messages are not old enough to be expired
        server.expire_old_messages_in_all_rooms();

        for room in server.rooms.values() {
            assert_eq!(room.messages.len(), 0);
        }
    }

    #[test]
    fn handle_new_connection_good_case() {
        let mut server = ServerState::new();
        let player_name = "some name".to_owned();
        let pkt = server.handle_new_connection(player_name, fake_socket_addr());
        match pkt {
            Packet::Response{sequence: _, request_ack: _, code} => {
                match code {
                    ResponseCode::LoggedIn(_,_) => {}
                    _ => panic!("Unexpected ResponseCode: {:?}", code)
                }
            }
            _ => panic!("Unexpected Packet Type: {:?}", pkt)
        }
    }

    #[test]
    fn handle_new_connection_player_name_taken() {
        let mut server = ServerState::new();
        let player_name = "some name".to_owned();

        let pkt = server.handle_new_connection(player_name.clone(), fake_socket_addr());
        match pkt {
            Packet::Response{sequence: _, request_ack: _, code} => {
                match code {
                    ResponseCode::LoggedIn(_,version) => {assert_eq!(version, VERSION.to_owned() )}
                    _ => panic!("Unexpected ResponseCode: {:?}", code)
                }
            }
            _ => panic!("Unexpected Packet Type: {:?}", pkt)
        }

        let pkt = server.handle_new_connection(player_name, fake_socket_addr());
        match pkt {
            Packet::Response{sequence: _, request_ack: _, code} => {
                match code {
                    ResponseCode::Unauthorized(msg) => { assert_eq!(msg, Some("not a unique name".to_owned())); }
                    _ => panic!("Unexpected ResponseCode: {:?}", code)
                }
            }
            _ => panic!("Unexpected Packet Type: {:?}", pkt)
        }
    }

    fn a_request_action_strat() -> BoxedStrategy<RequestAction> {
        prop_oneof![
            //Just(RequestAction::Disconnect), // not yet implemented
            //Just(RequestAction::KeepAlive),  // same
            Just(RequestAction::LeaveRoom),
            Just(RequestAction::ListPlayers),
            Just(RequestAction::ListRooms),
            Just(RequestAction::None),
        ].boxed()
    }

    fn a_request_action_complex_strat() -> BoxedStrategy<RequestAction> {
        prop_oneof![
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::ChatMessage(a)),
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::NewRoom(a)),
            ("([A-Z]{1,4} [0-9]{1,2}){3}").prop_map(|a| RequestAction::JoinRoom(a)),
            ("([A-Z]{1,4} [0-9]{1,2}){3}", "[0-9].[0-9].[0-9]").prop_map(|(a, b)| RequestAction::Connect{name: a, client_version: b})
        ].boxed()
    }

    // These tests are checking that we do not panic on each RequestAction
    proptest! {
        #[test]
        fn process_request_action_simple(ref request in a_request_action_strat()) {
            let mut server = ServerState::new();
            server.create_new_room(None, "some room".to_owned().clone());
            let player_id: PlayerID = {
                let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
                player.player_id
            };
            server.process_request_action(player_id, request.to_owned());
        }

        #[test]
        fn process_request_action_complex(ref request in a_request_action_complex_strat()) {
            let mut server = ServerState::new();
            server.create_new_room(None, "some room".to_owned().clone());
            let player_id: PlayerID = {
                let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
                player.player_id
            };
            server.process_request_action(player_id, request.to_owned());
        }
    }

    #[test]
    fn process_request_action_connect_while_connected() {
        let mut server = ServerState::new();
        server.create_new_room(None, "some room".to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        let result = server.process_request_action(player_id, RequestAction::Connect{name: "some player".to_owned(), client_version: "0.1.0".to_owned()});
        assert_eq!(result, ResponseCode::BadRequest(Some("already connected".to_owned())));
    }

    #[test]
    fn process_request_action_none_is_invalid() {
        let mut server = ServerState::new();
        server.create_new_room(None, "some room".to_owned().clone());
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.player_id
        };
        let result = server.process_request_action(player_id, RequestAction::None);
        assert_eq!(result, ResponseCode::BadRequest(Some("Invalid request".to_owned())));
    }

    #[test]
    fn prepare_response_spot_check_response_packet() {
        let mut server = ServerState::new();
        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.request_ack = Some(1);
            player.player_id
        };
        let pkt: Packet = server.prepare_response(player_id, RequestAction::ListRooms).unwrap();
        match pkt {
            Packet::Response{code, sequence, request_ack} => {
                assert_eq!(code, ResponseCode::RoomList(vec![]));
                assert_eq!(sequence, 1);
                assert_eq!(request_ack, Some(2));
            }
            _ => panic!("Unexpected Packet type on Response path: {:?}", pkt)
        }
        let player: &Player = server.get_player(player_id);
        assert_eq!(player.next_resp_seq, 2);
    }

    #[test]
    fn validate_client_version_client_is_up_to_date() {
        assert_eq!(validate_client_version( env!("CARGO_PKG_VERSION").to_owned()), true);
    }

    #[test]
    fn validate_client_version_client_is_very_old() {
      assert_eq!(validate_client_version("0.0.1".to_owned()), true);
    }

    #[test]
    fn validate_client_version_client_is_from_the_future() {
        assert_eq!(validate_client_version(format!("{}.{}.{}", <i32>::max_value(), <i32>::max_value(), <i32>::max_value()).to_owned()), false);
    }

    #[test]
    fn decode_packet_update_reply_good_case() {
        let mut server = ServerState::new();
        let cookie: String = {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.cookie.clone()
        };

        let update_reply_packet = Packet::UpdateReply {
                cookie: cookie,
                last_chat_seq: Some(0),
                last_game_update_seq: None,
                last_gen: None,
        };

        let result = server.decode_packet(fake_socket_addr(), update_reply_packet);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn decode_packet_update_reply_invalid_cookie() {
        let mut server = ServerState::new();
        {
            let player: &mut Player = server.add_new_player("some player".to_owned(), fake_socket_addr());
            player.cookie.clone()
        };

        let cookie = "CookieMonster".to_owned();

        let update_reply_packet = Packet::UpdateReply {
                cookie: cookie,
                last_chat_seq: Some(0),
                last_game_update_seq: None,
                last_gen: None,
        };

        let result = server.decode_packet(fake_socket_addr(), update_reply_packet);
        assert!(result.is_err());
    }

    #[test]
    fn construct_client_updates_no_rooms() {
        let mut server = ServerState::new();
        let result = server.construct_client_updates();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn construct_client_updates_empty_rooms() {
        let mut server = ServerState::new();
        server.create_new_room(None, "some room".to_owned().clone());
        let result = server.construct_client_updates();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn construct_client_updates_populated_room_returns_all_messages() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();
        let player_name = "some player".to_owned();
        let message_text = "Message".to_owned();

        server.create_new_room(None, room_name.clone());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        server.join_room(player_id, room_name);
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());
        let result = server.construct_client_updates();

        assert!(result.is_ok());
        let opt_output = result.unwrap();
        assert!(opt_output.is_some());
        let mut output: Vec<(SocketAddr, Packet)> = opt_output.unwrap();

        // Vector should contain a single item for this test
        assert_eq!(output.len(), 1);

        let (addr, pkt) = output.pop().unwrap();
        assert_eq!(addr, fake_socket_addr());

        match pkt {
            Packet::Update{chats, game_updates, universe_update} => {
                assert_eq!(game_updates, None);
                assert_eq!(universe_update, UniUpdateType::NoChange);
                assert!(chats.is_some());

                // All client chat sequence numbers start counting at 1
                let mut i=1;

                for msg in chats.unwrap() {
                    assert_eq!(msg.player_name, player_name);
                    assert_eq!(msg.chat_seq, Some(i));
                    assert_eq!(msg.message, message_text);
                    i+=1;
                }
            }
            _ => panic!("Unexpected packet in client update construction!")
        }
    }

    #[test]
    fn construct_client_updates_populated_room_returns_updates_after_client_acked() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();
        let player_name = "some player".to_owned();
        let message_text = "Message".to_owned();

        server.create_new_room(None, room_name.clone());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        server.join_room(player_id, room_name);
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());
        server.handle_chat_message(player_id, message_text.clone());

        // Assume that the client has acknowledged two chats
        {
            let player: &mut Player = server.get_player_mut(player_id);
            player.update_chat_seq_num(Some(2));
        }

        // We should then only return the last chat
        let result = server.construct_client_updates();

        assert!(result.is_ok());
        let opt_output = result.unwrap();
        assert!(opt_output.is_some());
        let mut output: Vec<(SocketAddr, Packet)> = opt_output.unwrap();

        // Vector should contain a single item for this test
        assert_eq!(output.len(), 1);

        let (addr, pkt) = output.pop().unwrap();
        assert_eq!(addr, fake_socket_addr());

        match pkt {
            Packet::Update{chats, game_updates, universe_update} => {
                assert_eq!(game_updates, None);
                assert_eq!(universe_update, UniUpdateType::NoChange);
                assert!(chats.is_some());

                let mut messages = chats.unwrap();
                assert_eq!(messages.len(), 1);
                let msg = messages.pop().unwrap();

                assert_eq!(msg.player_name, player_name);
                assert_eq!(msg.chat_seq, Some(3));
                assert_eq!(msg.message, message_text);
            }
            _ => panic!("Unexpected packet in client update construction!")
        }
    }

    #[test]
    fn broadcast_message_to_two_players_in_room() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();
        let player_name = "some player".to_owned();

        server.create_new_room(None, room_name.clone());

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };
        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.join_room(player_id, room_name.clone());
        {
            let room: &mut Room = server.get_room_mut(player_id).unwrap();
            room.broadcast("Silver birch against a Swedish sky".to_owned());
        }
        server.join_room(player_id2, room_name.clone());
        let room: &Room = server.get_room(player_id).unwrap();

        let player = (*server.get_player(player_id)).clone();
        let msgs = server.collect_unacknowledged_messages(room, &player).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message, "Silver birch against a Swedish sky".to_owned());

        let player = (*server.get_player(player_id2)).clone();
        let msgs = server.collect_unacknowledged_messages(room, &player).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message, "Silver birch against a Swedish sky".to_owned());
    }

    #[test]
    fn broadcast_message_to_an_empty_room() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();

        server.create_new_room(None, room_name.clone());
        let room_id: &RoomID = server.room_map.get(&room_name.clone()).unwrap();

        {
            let room: &mut Room = server.rooms.get_mut(&room_id).unwrap();
            room.broadcast("Silver birch against a Swedish sky".to_owned());
        }
        let room: &Room = server.rooms.get(&room_id).unwrap();
        assert_eq!(room.latest_seq_num, 1);
        assert_eq!(room.messages.len(), 1);
        let msgs: &ServerChatMessage = room.messages.get(0).unwrap();
        assert_eq!(msgs.player_name, "Server".to_owned());
        assert_eq!(msgs.seq_num, 1);
        assert_eq!(msgs.player_id, PlayerID(0xFFFFFFFFFFFFFFFF));
    }

    #[test]
    #[should_panic]
    fn disconnect_get_player_by_id_fails() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.handle_disconnect(player_id);
        server.get_player(player_id);
    }

    #[test]
    fn disconnect_get_player_by_cookie_fails() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, cookie) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            (player.player_id, player.cookie.clone())
        };

        server.handle_disconnect(player_id);
        assert_eq!(server.get_player_id_by_cookie(cookie.as_str()), None);
    }

    #[test]
    fn disconnect_while_in_room_removes_all_traces_of_player() {
        let mut server = ServerState::new();
        let room_name = "some_room".to_owned();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.player_id
        };

        server.create_new_room(None, room_name.clone());
        server.join_room(player_id, room_name);
        let room_id = {
            let room: &Room = server.get_room(player_id).unwrap();
            assert_eq!(room.player_ids.contains(&player_id), true);
            room.room_id
        };
        server.handle_disconnect(player_id);
        // Cannot go through player_id because the player has been removed
        let room: &Room = server.rooms.get(&room_id).unwrap();
        assert_eq!(room.player_ids.contains(&player_id), false);
    }

    #[test]
    fn test_is_previously_processed_packet() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(4);
            player.player_id
        };

        let player_id2: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = None;
            player.player_id
        };

        assert_eq!(server.is_previously_processed_packet(player_id2, 0), false);

        assert_eq!(server.is_previously_processed_packet(player_id, 0), true);
        assert_eq!(server.is_previously_processed_packet(player_id, 4), true);
        assert_eq!(server.is_previously_processed_packet(player_id, 5), false);
    }

    #[test]
    fn test_clear_transmission_queue_on_ack() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(4);
            player.player_id
        };

        for i in 0..5 {
            let pkt = Packet::Response {
                sequence: i,
                request_ack: None,
                code: ResponseCode::OK
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.tx_packets.buffer_item(pkt.clone());
        }

        server.clear_transmission_queue_on_ack(player_id, None);
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 5);
        server.clear_transmission_queue_on_ack(player_id, Some(0));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 5);
        server.clear_transmission_queue_on_ack(player_id, Some(1));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 4);
        server.clear_transmission_queue_on_ack(player_id, Some(5));
        assert_eq!(server.network_map.get(&player_id).unwrap().tx_packets.len(), 0);
    }

    #[test]
    fn test_resend_expired_tx_packets_empty_server() {
        let mut server = ServerState::new();

        let (udp_tx, _) = mpsc::unbounded();
        #[cfg(not(should_panic))]
        server.resend_expired_tx_packets(&udp_tx);
    }

    #[test]
    fn test_resend_expired_tx_packets() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let player_id: PlayerID = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(5);
            player.player_id
        };


        for i in 0..5 {
            let pkt = Packet::Response {
                sequence: i,
                request_ack: None,
                code: ResponseCode::OK
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.tx_packets.buffer_item(pkt.clone());

            if i < 3 {
                let attempt: &mut NetAttempt = nm.tx_packets.attempts.back_mut().unwrap();
                attempt.time = Instant::now() - Duration::from_secs(i+1);
            }
        }

        let (udp_tx, _) = mpsc::unbounded();
        server.resend_expired_tx_packets(&udp_tx);

        for i in 0..5 {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            let packet_retries: &NetAttempt = nm.tx_packets.attempts.get(i).unwrap();

            if i >= 3 {
                assert_eq!(packet_retries.retries, 0);
            } else {
                assert_eq!(packet_retries.retries, 1);
            }
        }
    }

    #[test]
    fn test_process_queued_rx_packets_first_non_connect_player_packet() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1);   // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        {
            let pkt = Packet::Request {
                cookie: Some(player_cookie),
                sequence: 2,
                response_ack: None,
                action: RequestAction::ListPlayers
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);


        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 1);
        }

    }

    #[test]
    fn test_process_queued_rx_packets_contiguous() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1);   // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        for i in 2..10 {
            let pkt = Packet::Request {
                cookie: Some(player_cookie.clone()),
                sequence: i,
                response_ack: None,
                action: RequestAction::ListPlayers
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);

        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 8);
        }

    }

    #[test]
    fn test_process_queued_rx_packets_swiss_cheese_queue() {
        let mut server = ServerState::new();
        let player_name = "some player".to_owned();

        let (player_id, player_cookie): (PlayerID, String) = {
            let player: &mut Player = server.add_new_player(player_name.clone(), fake_socket_addr());
            player.request_ack = Some(1);   // Player connected and we've confirmed the first transaction
            (player.player_id, player.cookie.clone())
        };

        for i in [2, 3, 4, 6, 8, 9, 10].iter() {
            let pkt = Packet::Request {
                cookie: Some(player_cookie.clone()),
                sequence: *i,
                response_ack: None,
                action: RequestAction::ListPlayers
            };

            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            nm.rx_packets.buffer_item(pkt.clone());

            assert_eq!(nm.tx_packets.len(), 0);
        }

        server.process_queued_rx_packets(player_id);

        {
            let nm: &mut NetworkManager = server.network_map.get_mut(&player_id).unwrap();
            assert_eq!(nm.tx_packets.len(), 3); // only 2, 3, and 4 are processed
        }
    }

}