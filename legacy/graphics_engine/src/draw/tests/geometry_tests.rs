//! Role: draw geometry tests.
//! Position: `draw/tests` in the graphics engine.
//! Signals & state: pure placement arithmetic.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::draw::geometry::{corner_uv, pack_offset, world_rect_rel};

// T-0xx Phase 1D: moved here with their subjects from the map engine's
// `renderers/batching/tests/lanes_tests.rs`. The anchor that was a private constant there is
// now the first argument, so the case that pinned the 12.8 km square passes it explicitly.
const ANCHOR: [f64; 2] = [6400.0, 6400.0];

#[test]
fn corner_uv_is_north_up() {
    assert_eq!(corner_uv([0.0, 1.0]), [0.0, 0.0]);
    assert_eq!(corner_uv([1.0, 1.0]), [1.0, 0.0]);
    assert_eq!(corner_uv([0.0, 0.0]), [0.0, 1.0]);
    assert_eq!(corner_uv([1.0, 0.0]), [1.0, 1.0]);
}

#[test]
fn pack_offset_places_north_at_top() {
    assert_eq!(pack_offset(2, 7, 2, 7), (0, 0));
    assert_eq!(pack_offset(4, 7, 2, 7), (512, 0));
    assert_eq!(pack_offset(2, 5, 2, 7), (0, 512));
    assert_eq!(pack_offset(3, 6, 2, 7), (256, 256));
}

#[test]
fn world_rect_rel_is_anchor_relative() {
    assert_eq!(
        world_rect_rel(ANCHOR, [0.0, 0.0], [12_800.0, 12_800.0]),
        [-6400.0, -6400.0, 6400.0, 6400.0]
    );
}
