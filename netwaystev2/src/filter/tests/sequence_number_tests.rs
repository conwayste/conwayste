use std::num::Wrapping;

use crate::filter::{SeqNumAdvancement, determine_seq_num_advancement};

#[test]
fn seq_num_not_yet_known() {
    let mut last_seen = None;

    assert_eq!(determine_seq_num_advancement(1, &mut last_seen), SeqNumAdvancement::BrandNew);
}

#[test]
fn seq_num_contiguous() {
    let mut last_seen = Some(Wrapping(0u64));

    assert_eq!(determine_seq_num_advancement(1, &mut last_seen), SeqNumAdvancement::Contiguous);
}

#[test]
fn seq_num_out_of_order() {
    let mut last_seen = Some(Wrapping(0u64));

    assert_eq!(determine_seq_num_advancement(2, &mut last_seen), SeqNumAdvancement::OutOfOrder);
}

#[test]
fn seq_num_duplicate() {
    let mut last_seen = Some(Wrapping(0u64));

    assert_eq!(determine_seq_num_advancement(0, &mut last_seen), SeqNumAdvancement::Duplicate);
}

#[test]
fn seq_num_wrapped_boundary_basic() {
    let mut last_seen = Some(Wrapping(u64::MAX));

    assert_eq!(determine_seq_num_advancement(0, &mut last_seen), SeqNumAdvancement::Contiguous);
}

#[test]
fn seq_num_wrapped_boundary_complex() {
    let mut last_seen = Some(Wrapping(u64::MAX - 1));

    assert_eq!(determine_seq_num_advancement(u64::MAX - 2, &mut last_seen), SeqNumAdvancement::Duplicate);
    assert_eq!(determine_seq_num_advancement(u64::MAX, &mut last_seen), SeqNumAdvancement::Contiguous);
    assert_eq!(determine_seq_num_advancement(1, &mut last_seen), SeqNumAdvancement::OutOfOrder);
    assert_eq!(determine_seq_num_advancement(0, &mut last_seen), SeqNumAdvancement::OutOfOrder);
    assert_eq!(determine_seq_num_advancement(2, &mut last_seen), SeqNumAdvancement::OutOfOrder);
}