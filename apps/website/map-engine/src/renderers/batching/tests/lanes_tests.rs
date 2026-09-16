//! Role: lanes tests.
//! Position: `renderers/batching/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::renderers::batching::lanes::*;

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
        world_rect_rel([0.0, 0.0], [12_800.0, 12_800.0]),
        [-6400.0, -6400.0, 6400.0, 6400.0]
    );
}

#[test]
fn grid_lines_everon_pinned() {
    let lines = grid_lines(12_800.0, 12_800.0, false);

    assert_eq!(lines.len(), 52);

    assert_eq!(lines[0].pos, [-6400.0, -6400.0]);
    assert_eq!(lines[0].color, norm(BORDER));

    assert_eq!(lines[1].pos, [-6400.0, 6400.0]);

    assert_eq!(lines[10].pos, [-1400.0, -6400.0]);
    assert_eq!(lines[10].color, norm(MAJOR));

    assert_eq!(lines[2].pos, [-5400.0, -6400.0]);
    assert_eq!(lines[2].color, norm(MINOR));

    assert_eq!(lines[26].pos, [-6400.0, -6400.0]);
    assert_eq!(lines[26].color, norm(BORDER));

    assert_eq!(lines[27].pos, [6400.0, -6400.0]);
}

#[test]
fn grid_lines_hs_palette() {
    let lines = grid_lines(12_800.0, 12_800.0, true);
    assert_eq!(lines[0].color, norm(BORDER_HS));
    assert_eq!(lines[10].color, norm(MAJOR_HS));
    assert_eq!(lines[2].color, norm(MINOR_HS));
}
