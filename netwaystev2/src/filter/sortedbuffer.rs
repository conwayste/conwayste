/// A sorted buffer over Packet which maintains packet ordering and uniqueness in the buffer.
use super::interface::{FilterMode, Packet};

use std::cmp::Reverse;
use std::collections::BinaryHeap;

// Using Reverse turns a BinaryHeap into a min-heap, perfect for returning lower ordered packets
type BufferedPacket = Reverse<Packet>;

impl From<Packet> for BufferedPacket {
    fn from(p: Packet) -> Self {
        Reverse(p)
    }
}

impl From<BufferedPacket> for Packet {
    fn from(bp: BufferedPacket) -> Self {
        bp.0
    }
}

impl<'a> From<&'a BufferedPacket> for &'a Packet {
    fn from(bp: &'a BufferedPacket) -> Self {
        &bp.0
    }
}

pub struct SortedBuffer {
    mode:     FilterMode,
    outgoing: BinaryHeap<BufferedPacket>,
    incoming: BinaryHeap<BufferedPacket>,
}

impl SortedBuffer {
    pub fn new(mode: FilterMode) -> Self {
        SortedBuffer {
            mode,
            outgoing: BinaryHeap::new(),
            incoming: BinaryHeap::new(),
        }
    }

    fn incoming_contains(&self, pkt: &Packet) -> bool {
        for pb in self.incoming.iter() {
            let packet: &Packet = pb.into();
            if packet == pkt {
                return true;
            }
        }
        false
    }

    fn outgoing_contains(&self, pkt: &Packet) -> bool {
        for pb in self.outgoing.iter() {
            let packet: &Packet = pb.into();
            if packet == pkt {
                return true;
            }
        }
        false
    }

    /// Add the incoming packet to the incoming queue (implemented as Binary Heap).
    /// Will silently drop packets if buffer's filter mode is not expected to handle the incoming packet type.
    pub fn incoming_push(&mut self, pkt: Packet) {
        match pkt {
            Packet::Request { .. } => {
                if self.mode == FilterMode::Server && !self.incoming_contains(&pkt) {
                    self.incoming.push(pkt.into());
                }
            }
            Packet::Response { .. } => {
                if self.mode == FilterMode::Client && !self.incoming_contains(&pkt) {
                    self.incoming.push(pkt.into());
                }
            }
            Packet::Update { .. } => {
                if self.mode == FilterMode::Client {
                    // TODO
                }
            }
            Packet::UpdateReply { .. } => {
                if self.mode == FilterMode::Server {
                    // TODO
                }
            }
            _ => { /* not handled */ }
        }
    }

    /// Add the outgoing packet to the outgoing queue (implemented as Binary Heap).
    /// Will silently drop packets if buffer's filter mode is not expected to handle the outgoing packet type.
    pub fn outgoing_push(&mut self, pkt: Packet) {
        match pkt {
            Packet::Request { .. } => {
                if self.mode == FilterMode::Client && !self.outgoing_contains(&pkt) {
                    self.outgoing.push(pkt.into());
                }
            }
            Packet::Response { .. } => {
                if self.mode == FilterMode::Server && !self.outgoing_contains(&pkt) {
                    self.outgoing.push(pkt.into());
                }
            }
            Packet::Update { .. } => {
                if self.mode == FilterMode::Server {
                    // TODO
                }
            }
            Packet::UpdateReply { .. } => {
                if self.mode == FilterMode::Client {
                    // TODO
                }
            }
            _ => { /* not handled */ }
        }
    }
    /// Removes one packet from the incoming queue (implemented as Binary Heap).
    pub fn incoming_pop(&mut self) -> Option<Packet> {
        self.incoming.pop().map(|bp| bp.into())
    }

    /// Removes one packet from the outgoing queue (implemented as Binary Heap).
    pub fn outgoing_pop(&mut self) -> Option<Packet> {
        self.outgoing.pop().map(|bp| bp.into())
    }
}
