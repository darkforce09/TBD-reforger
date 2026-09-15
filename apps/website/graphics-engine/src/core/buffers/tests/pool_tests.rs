//! Role: pool tests.
//! Position: `core/buffers/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::buffers::pool::*;

#[test]
fn equal_size_uploads_allocate_once() {
    let mut pool = LanePool::new();
    let bytes = [7u8; 20];
    pool.write(1, &bytes);
    pool.write(1, &bytes);
    assert_eq!(
        pool.creates(),
        1,
        "two equal-size uploads must create one buffer"
    );
}

#[test]
fn growth_keeps_content() {
    let mut pool = LanePool::new();
    pool.write(1, &[1, 2, 3, 4]);
    assert_eq!(pool.creates(), 1);
    pool.write(1, &[9, 8, 7, 6, 5, 4, 3, 2]);
    assert_eq!(pool.creates(), 2);
    assert!(
        pool.capacity(1) >= 8,
        "grown capacity must cover the larger write"
    );
    assert_eq!(pool.contents(1), &[9, 8, 7, 6, 5, 4, 3, 2]);
}

#[test]
fn smaller_write_reuses_and_never_shrinks() {
    let mut pool = LanePool::new();
    pool.write(1, &[3u8; 64]);
    let cap = pool.capacity(1);
    assert_eq!(pool.creates(), 1);
    pool.write(1, &[1, 2, 3, 4]);
    assert_eq!(pool.creates(), 1, "smaller write must reuse the buffer");
    assert_eq!(pool.capacity(1), cap, "capacity must never shrink");
    assert_eq!(pool.contents(1), &[1, 2, 3, 4]);
}

#[test]
fn grow_capacity_doubles_and_never_shrinks() {
    assert_eq!(grow_capacity(0, 20), 20);
    assert_eq!(grow_capacity(20, 20), 20);
    assert_eq!(grow_capacity(20, 21), 40);
    assert_eq!(grow_capacity(40, 4), 40);
}
