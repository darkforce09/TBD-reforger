//! Role: world tests.
//! Position: `spatial/indexing/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::indexing::world::*;

const B: u8 = 0;
const T: u8 = 1;

fn idx() -> WorldSpatialIndex {
    let mut ix = WorldSpatialIndex::new();

    ix.insert_chunk("0_0", &[10.0, 12.0], &[10.0, 10.0], &[B, T]);

    ix.insert_chunk("1_0", &[600.0], &[10.0], &[B]);
    ix
}

#[test]
fn pick_rect_class_filter() {
    let mut ix = idx();
    let mut all = ix.pick_rect(0.0, 0.0, 20.0, 20.0, None);
    all.sort();
    assert_eq!(all, vec!["0_0:0", "0_0:1"]);

    let only_b = ix.pick_rect(0.0, 0.0, 20.0, 20.0, Some(1 << B));
    assert_eq!(only_b, vec!["0_0:0"]);
}

#[test]
fn pick_nearest_circular_and_class() {
    let mut ix = idx();

    assert_eq!(ix.pick_nearest(10.4, 10.0, 5.0, None), Some("0_0:0".into()));

    assert_eq!(
        ix.pick_nearest(10.4, 10.0, 5.0, Some(1 << T)),
        Some("0_0:1".into())
    );

    assert_eq!(ix.pick_nearest(10.4, 10.0, 0.1, None), None);
}

#[test]
fn no_class_rows_are_skipped() {
    let mut ix = WorldSpatialIndex::new();
    ix.insert_chunk("0_0", &[1.0, 2.0], &[1.0, 2.0], &[B, NO_CLASS]);
    assert_eq!(ix.size(), 1);
    assert_eq!(ix.pick_rect(0.0, 0.0, 10.0, 10.0, None), vec!["0_0:0"]);
}

#[test]
fn remove_and_reinsert_are_idempotent() {
    let mut ix = idx();
    assert_eq!(ix.size(), 3);
    ix.remove_chunk("0_0");
    assert_eq!(ix.size(), 1);
    assert!(ix.pick_rect(0.0, 0.0, 20.0, 20.0, None).is_empty());

    ix.insert_chunk("1_0", &[600.0], &[10.0], &[B]);
    assert_eq!(ix.size(), 1);
}
