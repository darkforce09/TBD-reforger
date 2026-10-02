//! Role: cartographic strip.
//! Position: `world/terrain/roads` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::roads::styling::StripVertex;
use crate::world::terrain::roads::styling::expand_polyline_strip;
use crate::world::terrain::roads::styling::norm_rgba;
use crate::world::terrain::roads::styling::pack_strip_verts;

/// Canonical fence strip width m value.
pub const FENCE_STRIP_WIDTH_M: f64 = 0.35;

/// Canonical strip min px value.
pub const STRIP_MIN_PX: f64 = 1.5;

/// Canonical pier strip max width m value.
pub const PIER_STRIP_MAX_WIDTH_M: f64 = 6.0;

/// Canonical bridge railing radius m value.
pub const BRIDGE_RAILING_RADIUS_M: f64 = 8.0;

/// Cartographic neutral fence/railing stroke `#8a8478` @ α0.85.
pub const FENCE_STRIP_RGBA: [u8; 4] = [0x8a, 0x84, 0x78, 217];

/// Clamp strip width m.
#[must_use]
pub fn clamp_strip_width_m(base_width_m: f64, deck_zoom: f64) -> f64 {
    base_width_m.max(STRIP_MIN_PX / 2.0_f64.powf(deck_zoom))
}

/// Obb long axis endpoints.
#[must_use]
pub fn obb_long_axis_endpoints(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
) -> [[f64; 2]; 2] {
    let c =
        crate::world::environment::buildings::obb::obb_corners(x, y, half_x, half_y, rotation_deg);
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];

    let a0 = mid(c[3], c[0]);
    let a1 = mid(c[1], c[2]);

    let b0 = mid(c[0], c[1]);
    let b1 = mid(c[2], c[3]);
    let d2 = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
    if d2(a0, a1) >= d2(b0, b1) {
        [a0, a1]
    } else {
        [b0, b1]
    }
}

/// Expand a thin strip along the OBB long axis; `width_m` is the full stroke width.
#[must_use]
pub fn compose_obb_strip(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    width_m: f64,
    color: [f32; 4],
) -> Vec<StripVertex> {
    let pts = obb_long_axis_endpoints(x, y, half_x, half_y, rotation_deg);
    expand_polyline_strip(&pts, width_m, color)
}

/// Fence prop strip at [`FENCE_STRIP_WIDTH_M`], floored to [`STRIP_MIN_PX`] at `deck_zoom`.
#[must_use]
pub fn compose_fence_strip(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    deck_zoom: f64,
) -> Vec<StripVertex> {
    let width_m = clamp_strip_width_m(FENCE_STRIP_WIDTH_M, deck_zoom);
    compose_obb_strip(
        x,
        y,
        half_x,
        half_y,
        rotation_deg,
        width_m,
        norm_rgba(FENCE_STRIP_RGBA),
    )
}

/// Compose pier strip.
#[must_use]
pub fn compose_pier_strip(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    fill_rgba: [u8; 4],
    deck_zoom: f64,
) -> Vec<StripVertex> {
    let base = (half_x.min(half_y) * 2.0).min(PIER_STRIP_MAX_WIDTH_M);
    let width_m = clamp_strip_width_m(base, deck_zoom);
    compose_obb_strip(
        x,
        y,
        half_x,
        half_y,
        rotation_deg,
        width_m,
        norm_rgba(fill_rgba),
    )
}

/// Compose bridge rail strips.
#[must_use]
pub fn compose_bridge_rail_strips(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    deck_zoom: f64,
) -> Vec<StripVertex> {
    let [p0, p1] = obb_long_axis_endpoints(x, y, half_x, half_y, rotation_deg);
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len = dx.hypot(dy).max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let (px, py) = (-uy, ux);
    let off = half_x.min(half_y).min(BRIDGE_RAILING_RADIUS_M);
    let width_m = clamp_strip_width_m(FENCE_STRIP_WIDTH_M, deck_zoom);
    let color = norm_rgba(FENCE_STRIP_RGBA);
    let mut out = Vec::new();
    for s in [1.0_f64, -1.0] {
        let a = [p0[0] + px * off * s, p0[1] + py * off * s];
        let b = [p1[0] + px * off * s, p1[1] + py * off * s];
        out.extend(expand_polyline_strip(&[a, b], width_m, color));
    }
    out
}

/// Pack strip verts into flat `[x,y,r,g,b,a]…` for the render engine.
#[must_use]
pub fn pack_cartographic_strips(verts: &[StripVertex]) -> Vec<f32> {
    pack_strip_verts(verts)
}

/// Midpoint world width of a strip at a segment (Class R gate G4).
#[must_use]
pub fn strip_world_width_at_midpoint(verts: &[StripVertex]) -> Option<f64> {
    if verts.len() < 2 {
        return None;
    }
    let a = verts[0].pos;
    let b = verts[1].pos;
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    Some(dx.hypot(dy))
}

#[cfg(test)]
#[path = "tests/cartographic_strip_tests.rs"]
mod tests;
