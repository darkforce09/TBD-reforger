//! Role: draw geometry.
//! Position: `draw` in the graphics engine.
//! Signals & state: one line vertex, and the arithmetic that places a rect against an anchor.
//! Invariants: the anchor arrives as an argument. This crate holds no world origin of its own
//! — which point large f64 coordinates are folded against is the caller's fact about its map.

use bytemuck::{Pod, Zeroable};

/// One grid-line vertex: anchor-relative position (meters) + normalized-RGBA color. 24 B, laid out for the Polyline vertex buffer (`@location(0) pos`, `@location(1) color`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    /// Anchor-relative [x, y], meters (world minus the caller's anchor).
    pub pos: [f32; 2],

    /// RGBA, linear 0..1 (non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// RGBA8 → linear 0..1 floats.
#[must_use]
pub(crate) fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

/// World meters → anchor-relative f32 meters.
///
/// The fold to f32 is why the anchor exists: subtracting it first keeps the magnitude small
/// enough that f32 rounding stays far under a pixel at every zoom.
#[must_use]
pub(crate) fn rel(anchor: [f64; 2], x: f64, y: f64) -> [f32; 2] {
    #[allow(clippy::cast_possible_truncation)]
    [(x - anchor[0]) as f32, (y - anchor[1]) as f32]
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

/// Anchor-relative-meters `[minX, minY, maxX, maxY]` (f32) for a world rect — the textured-quad instance geometry, matching the [`crate::draw::instances::QuadInstance`] anchor contract.
#[must_use]
pub fn world_rect_rel(anchor: [f64; 2], min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    let a = rel(anchor, min[0], min[1]);
    let b = rel(anchor, max[0], max[1]);
    [a[0], a[1], b[0], b[1]]
}

#[cfg(test)]
#[path = "tests/geometry_tests.rs"]
mod tests;
