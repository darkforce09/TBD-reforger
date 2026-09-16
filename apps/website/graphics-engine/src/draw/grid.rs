//! Role: draw grid.
//! Position: `draw` in the graphics engine.
//! Signals & state: a procedural `LineList` over a rect, at a fixed step.
//! Invariants: a rect, a step and two palettes. Which rect, and what the ground under it is,
//! never reaches this module — both arrive as arguments.

use crate::draw::geometry::{LineVertex, norm, rel};

const GRID_STEP: u32 = 1000;

const MAJOR_STEP: u32 = 5000;

const MINOR: [u8; 4] = [173, 198, 255, 28];
const MAJOR: [u8; 4] = [173, 198, 255, 60];
const BORDER: [u8; 4] = [173, 198, 255, 90];
const MINOR_HS: [u8; 4] = [173, 198, 255, 80];
const MAJOR_HS: [u8; 4] = [173, 198, 255, 150];
const BORDER_HS: [u8; 4] = [173, 198, 255, 210];

/// Build the procedural 1 km grid as a `LineList` vertex buffer (2 vertices per line), an exact mirror of `useBaseMapLayer.ts:44-58`: verticals `x ∈ [0, width]` step 1000 (`x <= width` inclusive) then horizontals `y ∈ [0, height]`; color is BORDER (`x == 0 || x >= width`) / MAJOR (`x % 5000 == 0`) / MINOR (else), switching to the `_HS` palette when the caller says a shaded layer sits underneath.
#[must_use]
pub fn grid_lines(
    anchor: [f64; 2],
    width: f64,
    height: f64,
    over_hillshade: bool,
) -> Vec<LineVertex> {
    let (minor, major, border) = if over_hillshade {
        (norm(MINOR_HS), norm(MAJOR_HS), norm(BORDER_HS))
    } else {
        (norm(MINOR), norm(MAJOR), norm(BORDER))
    };
    let mut out = Vec::new();

    let mut push_line = |a: [f32; 2], b: [f32; 2], color: [f32; 4]| {
        out.push(LineVertex { pos: a, color });
        out.push(LineVertex { pos: b, color });
    };

    let mut x: u32 = 0;
    while f64::from(x) <= width {
        let on_border = x == 0 || f64::from(x) >= width;
        let color = if on_border {
            border
        } else if x.is_multiple_of(MAJOR_STEP) {
            major
        } else {
            minor
        };
        push_line(
            rel(anchor, f64::from(x), 0.0),
            rel(anchor, f64::from(x), height),
            color,
        );
        x += GRID_STEP;
    }

    let mut y: u32 = 0;
    while f64::from(y) <= height {
        let on_border = y == 0 || f64::from(y) >= height;
        let color = if on_border {
            border
        } else if y.is_multiple_of(MAJOR_STEP) {
            major
        } else {
            minor
        };
        push_line(
            rel(anchor, 0.0, f64::from(y)),
            rel(anchor, width, f64::from(y)),
            color,
        );
        y += GRID_STEP;
    }
    out
}

#[cfg(test)]
#[path = "tests/grid_tests.rs"]
mod tests;
