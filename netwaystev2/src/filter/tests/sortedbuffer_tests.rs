use crate::filter::SequencedMinHeap;

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
    let span: Vec<u64> = vec![1, 2, 5, 6, 9];

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
    let mut span: Vec<u64> = vec![1, 2, 5, 6, 9];
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
    let mut span: Vec<u64> = vec![2, 9, 1, 6, 5];

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
    let mut span: Vec<u64> = vec![2, 6, 1, 3, 5, 0, 4];
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
