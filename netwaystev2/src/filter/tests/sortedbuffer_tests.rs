use crate::filter::sortedbuffer::SequencedMinHeap;

#[test]
fn seqminheap_empty_checks() {
    let mut smh = SequencedMinHeap::<usize>::new();

    assert_eq!(smh.contains_sequence_number(0), false);
    assert_eq!(smh.take_if_matching(0), None);
    assert_eq!(smh.take_if_matching(1), None);
    assert_eq!(smh.take_if_matching(0x8000000000000000), None);
    assert_eq!(smh.take_if_matching(0xffffffffffffffff), None);
}

#[test]
fn seqminheap_filled_checks() {
    let mut smh = SequencedMinHeap::<usize>::new();

    for i in 2..4 {
        smh.add(i, i as usize);
    }

    assert_eq!(smh.contains_sequence_number(0), false);
    assert_eq!(smh.take_if_matching(4), None);
    assert_eq!(smh.take_if_matching(3), None);
    assert_eq!(smh.take_if_matching(2), Some(2));
    assert_eq!(smh.take_if_matching(3), Some(3));
}

#[test]
fn seqminheap_insert_ascending() {
    let mut smh = SequencedMinHeap::<usize>::new();
    for i in 0..10 {
        smh.add(i, i as usize);
    }

    for i in 0..10 {
        assert_eq!(smh.contains_sequence_number(i), true);
    }

    for i in 0..10 {
        assert_eq!(smh.take_if_matching(i), Some(i as usize));
    }
}

#[test]
fn seqminheap_insert_ascending_with_wrapping() {
    let mut smh = SequencedMinHeap::<usize>::new();

    smh.add(u64::max_value(), usize::max_value());
    for i in 0..10 {
        smh.add(i, i as usize);
    }

    assert_eq!(smh.contains_sequence_number(u64::max_value()), true);
    for i in 0..10 {
        assert_eq!(smh.contains_sequence_number(i), true);
    }

    assert_eq!(smh.take_if_matching(u64::max_value()), Some(usize::max_value()));
    for i in 0..10 {
        assert_eq!(smh.take_if_matching(i), Some(i as usize));
    }
}

#[test]
fn seqminheap_insert_descending() {
    let mut smh = SequencedMinHeap::<usize>::new();
    for i in (0..10).rev() {
        smh.add(i, i as usize);
    }

    for i in 0..10 {
        assert_eq!(smh.contains_sequence_number(i), true);
    }

    for i in 0..10 {
        assert_eq!(smh.take_if_matching(i), Some(i as usize));
    }
}

#[test]
fn seqminheap_insert_out_of_order_with_no_gaps() {
    let mut span: Vec<u64> = vec![2, 6, 1, 3, 5, 0, 4];
    span.reverse();

    let mut smh = SequencedMinHeap::<usize>::new();
    for n in &span {
        smh.add(*n, *n as usize);
    }

    for x in &span {
        assert_eq!(smh.contains_sequence_number(*x), true);
    }

    span.sort();
    for x in &span {
        assert_eq!(smh.take_if_matching(*x), Some(*x as usize));
    }
}
