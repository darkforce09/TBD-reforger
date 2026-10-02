//! Role: draw geometry tests.
//! Position: `draw/tests` in `render_primitives`.
//! Signals & state: pure placement arithmetic.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::draw::geometry::{corner_uv, pack_offset, world_rect_rel};

// The anchor is an argument of every placement function; the cases pass this one explicitly.
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
