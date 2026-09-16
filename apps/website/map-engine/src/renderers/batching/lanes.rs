//! Role: lanes.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::renderers::batching::scene::ANCHOR;
use bytemuck::{Pod, Zeroable};

/// One grid-line vertex: anchor-relative position (meters) + normalized-RGBA color. 24 B, laid out for the Polyline vertex buffer (`@location(0) pos`, `@location(1) color`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    /// Anchor-relative [x, y], meters (world minus [`ANCHOR`]).
    pub pos: [f32; 2],

    /// RGBA, linear 0..1 (non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

const GRID_STEP: u32 = 1000;

const MAJOR_STEP: u32 = 5000;

const MINOR: [u8; 4] = [173, 198, 255, 28];
const MAJOR: [u8; 4] = [173, 198, 255, 60];
const BORDER: [u8; 4] = [173, 198, 255, 90];
const MINOR_HS: [u8; 4] = [173, 198, 255, 80];
const MAJOR_HS: [u8; 4] = [173, 198, 255, 150];
const BORDER_HS: [u8; 4] = [173, 198, 255, 210];

#[must_use]
fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

#[must_use]
fn rel(x: f64, y: f64) -> [f32; 2] {
    #[allow(clippy::cast_possible_truncation)]
    [(x - ANCHOR[0]) as f32, (y - ANCHOR[1]) as f32]
}

/// Build the procedural 1 km grid as a `LineList` vertex buffer (2 vertices per line), an exact mirror of `useBaseMapLayer.ts:44-58`: verticals `x ∈ [0, width]` step 1000 (`x <= width` inclusive) then horizontals `y ∈ [0, height]`; color is BORDER (`x == 0 || x >= width`) / MAJOR (`x % 5000 == 0`) / MINOR (else), switching to the `_HS` palette over the hillshade.
#[must_use]
pub fn grid_lines(width: f64, height: f64, over_hillshade: bool) -> Vec<LineVertex> {
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
        push_line(rel(f64::from(x), 0.0), rel(f64::from(x), height), color);
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
        push_line(rel(0.0, f64::from(y)), rel(width, f64::from(y)), color);
        y += GRID_STEP;
    }
    out
}

/// North-up UV for a textured quad: unit `(x, y)` over the world rect → texture `(u, v)`. Unit `y = 1` is the world maxY (north); the texture's top row (`v = 0`) is north (Deck BitmapLayer, TBDS block-row-0-north, Rust hillshade row-0-north all agree), so `v = 1 - unit.y`.
#[must_use]
pub fn corner_uv(unit: [f32; 2]) -> [f32; 2] {
    [unit[0], 1.0 - unit[1]]
}

/// Pack offset.
#[must_use]
pub fn pack_offset(tx: u32, ty: u32, tx_min: u32, ty_max: u32) -> (u32, u32) {
    ((tx - tx_min) * 256, (ty_max - ty) * 256)
}

/// Anchor-relative-meters `[minX, minY, maxX, maxY]` (f32) for a world rect — the textured-quad instance geometry, matching the `scene::QuadInstance` anchor contract.
#[must_use]
pub fn world_rect_rel(min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    let a = rel(min[0], min[1]);
    let b = rel(max[0], max[1]);
    [a[0], a[1], b[0], b[1]]
}

#[cfg(test)]
#[path = "tests/lanes_tests.rs"]
mod tests;
