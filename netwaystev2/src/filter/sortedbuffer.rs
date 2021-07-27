///! A sorted buffer which maintains ordering and uniqueness in the buffer.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub struct SequencedMinHeap<T> {
    heap: BinaryHeap<Reverse<SequencedItem<T>>>,
}

impl<T> SequencedMinHeap<T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    fn contains_sequence_number(&self, sequence: u64) -> bool {
        // Searching takes place in arbitrary order
        for tuple in &self.heap {
            if tuple.0.0 == sequence {
                return true
            }
        }
        false
    }

    /// Add this T to the sequenced min-heap. Returns false if not added because
    /// this sequence is already in the min-heap; otherwise, returns true.
    pub fn add(&mut self, sequence: u64, t: T) -> bool {
        if self.contains_sequence_number(sequence) {
            return false;
        }
        self.heap.push(Reverse(SequencedItem(sequence, t)));
        true
    }

    /// Gets the minimum sequence number in the min-heap. This is the sequence number of
    /// what we would .take()
    pub fn peek_sequence_number(&self) -> Option<u64> {
        self.heap.peek().map(|reversed_tup| reversed_tup.0.0)
    }

    /// Takes the T with the lowest sequence number
    pub fn take(&mut self) -> Option<T> {
        self.heap.pop().map(|reversed_tup| reversed_tup.0.1)
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

#[cfg(test)]
mod test {
    use super::SequencedMinHeap;

    #[test]
    fn seqminheap_empty_checks() {
        let smh = SequencedMinHeap::<usize>::new();

        assert_eq!(smh.contains_sequence_number(0), false);
        assert_eq!(smh.peek_sequence_number(), None);
    }

    #[test]
    fn seqminheap_filled_checks() {
        let mut smh = SequencedMinHeap::<usize>::new();

        for i in 2..4 {
            smh.add(i, 0);
        }

        assert_eq!(smh.contains_sequence_number(0), false);
        assert_eq!(smh.peek_sequence_number(), Some(2));
    }

/*
 * The next few tests fail because the underlying BinaryHeap makes no guarantee about iteration order. It's arbitrary.
 */

    #[test]
    fn seqminheap_insert_ascending() {
        let mut smh = SequencedMinHeap::<usize>::new();
        for i in 0..10 {
            smh.add(i, 0);
        }

        for i in 0..10 {
            assert_eq!(smh.contains_sequence_number(i), true);
        }

        for i in 0..10 {
            assert_eq!(smh.peek_sequence_number(), Some(i));
            let _ = smh.take();
        }
    }

    #[test]
    fn seqminheap_insert_descending() {
        let mut smh = SequencedMinHeap::<usize>::new();
        for i in (0..10).rev() {
            smh.add(i, 0);
        }

        for i in 0..10 {
            assert_eq!(smh.contains_sequence_number(i), true);
        }

        for i in 0..10 {
            assert_eq!(smh.peek_sequence_number(), Some(i));
            let _ = smh.take();
        }
    }

    #[test]
    fn seqminheap_insert_sequential_with_gaps() {
        let span : Vec<u64> = vec![1, 2, 5, 6, 9];

        let mut smh = SequencedMinHeap::<usize>::new();
        for n in &span {
            smh.add(*n, 0);
        }

        for x in &span {
            assert_eq!(smh.contains_sequence_number(*x), true);
        }

        for x in &span {
            assert_eq!(smh.peek_sequence_number(), Some(*x));
            let _ = smh.take();
        }
    }

    #[test]
    fn seqminheap_insert_reverse_sequential_with_gaps() {
        let mut span : Vec<u64> = vec![1, 2, 5, 6, 9];
        span.reverse();

        let mut smh = SequencedMinHeap::<usize>::new();
        for n in &span {
            smh.add(*n, 0);
        }

        for x in &span {
            assert_eq!(smh.contains_sequence_number(*x), true);
        }

        span.reverse();
        for x in &span {
            assert_eq!(smh.peek_sequence_number(), Some(*x));
            let _ = smh.take();
        }
    }

    #[test]
    fn seqminheap_insert_out_of_order_with_gaps() {
        let mut span : Vec<u64> = vec![2, 9, 1, 6, 5];

        let mut smh = SequencedMinHeap::<usize>::new();
        for n in &span {
            smh.add(*n, 0);
        }

        for x in &span {
            assert_eq!(smh.contains_sequence_number(*x), true);
        }

        span.sort();
        for x in &span {
            assert_eq!(smh.peek_sequence_number(), Some(*x));
            let _ = smh.take();
        }
    }

    #[test]
    fn seqminheap_insert_out_of_order_with_no_gaps() {
        let mut span : Vec<u64> = vec![2, 6, 1, 3, 5, 0, 4];
        span.reverse();

        let mut smh = SequencedMinHeap::<usize>::new();
        for n in &span {
            smh.add(*n, 0);
        }

        for x in &span {
            assert_eq!(smh.contains_sequence_number(*x), true);
        }

        span.sort();
        for x in &span {
            assert_eq!(smh.peek_sequence_number(), Some(*x));
            let _ = smh.take();
        }
    }
}