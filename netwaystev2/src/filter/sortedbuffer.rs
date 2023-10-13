///! A sorted buffer which maintains ordering and uniqueness in the buffer.
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub struct SequencedMinHeap<T> {
    heap0: BinaryHeap<Reverse<SequencedItem<T>>>,
    heap1: BinaryHeap<Reverse<SequencedItem<T>>>,
}

impl<T> SequencedMinHeap<T> {
    pub fn new() -> Self {
        Self {
            heap0: BinaryHeap::new(), // For sequence numbers with most significant bit equal to 0
            heap1: BinaryHeap::new(), // Otherwise (MSB == 1) (to handle wrapping properly)
        }
    }

    pub fn contains_sequence_number(&self, sequence: u64) -> bool {
        // Searching takes place in arbitrary order
        match Location::get(sequence) {
            Location::Heap0 => {
                for tuple in &self.heap0 {
                    if tuple.0 .0 == sequence {
                        return true;
                    }
                }
                false
            }
            Location::Heap1 => {
                for tuple in &self.heap1 {
                    if tuple.0 .0 == sequence {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Add this T to the sequenced min-heap. Returns false if not added because
    /// this sequence is already in the min-heap; otherwise, returns true.
    pub fn add(&mut self, sequence: u64, t: T) -> bool {
        if self.contains_sequence_number(sequence) {
            return false;
        }
        match Location::get(sequence) {
            Location::Heap0 => self.heap0.push(Reverse(SequencedItem(sequence, t))),
            Location::Heap1 => self.heap1.push(Reverse(SequencedItem(sequence, t))),
        }
        true
    }

    /// Takes the T if the next sequence number we are expecting is `expected_sequence`.
    pub fn take_if_matching(&mut self, expected_sequence: u64) -> Option<T> {
        match Location::get(expected_sequence) {
            Location::Heap0 => {
                if let Some(reversed_tup) = self.heap0.peek() {
                    if expected_sequence == reversed_tup.0 .0 {
                        return self.heap0.pop().map(|reversed_tup| reversed_tup.0 .1);
                    }
                }
            }
            Location::Heap1 => {
                if let Some(reversed_tup) = self.heap1.peek() {
                    if expected_sequence == reversed_tup.0 .0 {
                        return self.heap1.pop().map(|reversed_tup| reversed_tup.0 .1);
                    }
                }
            }
        }
        None
    }
}

struct SequencedItem<T>(u64, T);

impl<T> Ord for SequencedItem<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> PartialEq for SequencedItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> PartialOrd for SequencedItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Marker trait impl for Ord without needing to pass `Eq` down to the item
impl<T> Eq for SequencedItem<T> {
}

enum Location {
    Heap0,
    Heap1,
}

impl Location {
    fn get(sequence: u64) -> Self {
        if sequence & (1 << 63) == (1 << 63) {
            Location::Heap1
        } else {
            Location::Heap0
        }
    }
}
