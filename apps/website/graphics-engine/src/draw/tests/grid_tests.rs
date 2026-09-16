//! Role: draw grid tests.
//! Position: `draw/tests` in the graphics engine.
//! Signals & state: the procedural grid's vertex stream.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::draw::geometry::norm;
use crate::draw::grid::*;

// T-0xx Phase 1D: moved here with `grid_lines` from the map engine's
// `renderers/batching/tests/lanes_tests.rs`. The anchor is now an argument; the pinned
// vertices below are the same bytes, with the 12.8 km square's centre passed explicitly.
const ANCHOR: [f64; 2] = [6400.0, 6400.0];

#[test]
fn grid_lines_pinned() {
    let lines = grid_lines(ANCHOR, 12_800.0, 12_800.0, false);

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
    let lines = grid_lines(ANCHOR, 12_800.0, 12_800.0, true);
    assert_eq!(lines[0].color, norm(BORDER_HS));
    assert_eq!(lines[10].color, norm(MAJOR_HS));
    assert_eq!(lines[2].color, norm(MINOR_HS));
}
