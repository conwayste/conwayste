use std::num::Wrapping;

use crate::filter::{determine_seq_num_advancement, SeqNumAdvancement};

#[test]
fn seq_num_not_yet_known() {
    let last_seen = None;

    assert_eq!(determine_seq_num_advancement(1, last_seen), SeqNumAdvancement::BrandNew);
}

#[test]
fn seq_num_contiguous() {
    let last_seen = Some(Wrapping(0u64));

    assert_eq!(
        determine_seq_num_advancement(1, last_seen),
        SeqNumAdvancement::Contiguous
    );
}

#[test]
fn seq_num_out_of_order() {
    let last_seen = Some(Wrapping(0u64));

    assert_eq!(
        determine_seq_num_advancement(2, last_seen),
        SeqNumAdvancement::OutOfOrder
    );
}

#[test]
fn seq_num_duplicate() {
    let last_seen = Some(Wrapping(0u64));

    assert_eq!(
        determine_seq_num_advancement(0, last_seen),
        SeqNumAdvancement::Duplicate
    );
}

#[test]
fn seq_num_wrapped_boundary_basic() {
    let last_seen = Some(Wrapping(u64::MAX));

    assert_eq!(
        determine_seq_num_advancement(0, last_seen),
        SeqNumAdvancement::Contiguous
    );
}

#[test]
fn seq_num_wrapped_boundary_complex() {
    let last_seen = Some(Wrapping(u64::MAX - 1));

    assert_eq!(
        determine_seq_num_advancement(u64::MAX - 2, last_seen),
        SeqNumAdvancement::Duplicate
    );
    assert_eq!(
        determine_seq_num_advancement(u64::MAX, last_seen),
        SeqNumAdvancement::Contiguous
    );
    assert_eq!(
        determine_seq_num_advancement(1, last_seen),
        SeqNumAdvancement::OutOfOrder
    );
    assert_eq!(
        determine_seq_num_advancement(0, last_seen),
        SeqNumAdvancement::OutOfOrder
    );
    assert_eq!(
        determine_seq_num_advancement(2, last_seen),
        SeqNumAdvancement::OutOfOrder
    );
}
