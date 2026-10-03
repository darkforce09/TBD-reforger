//! The thin strips the map draws along a footprint's long axis: fences, piers and bridge rails.
//!
//! **Role:** finds a footprint's long axis from its oriented corners ([`obb_long_axis_endpoints`])
//! and expands it into fence, pier and bridge-rail strips ([`compose_fence_strip`],
//! [`compose_pier_strip`], [`compose_bridge_rail_strips`]), clamped so no strip draws narrower than
//! [`STRIP_MIN_PX`] pixels.
//! **Position:** reads `prefab_catalog`'s footprint corners and [`crate::styling`]'s strip
//! expansion; the map engine's strip packer calls it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a strip runs along the longer of the footprint's two axes; its world width is
//! never below the [`STRIP_MIN_PX`] floor at the zoom it is built for.

use crate::styling::StripVertex;
use crate::styling::expand_polyline_strip;
use crate::styling::norm_rgba;
use crate::styling::pack_strip_verts;

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

/// `base_width_m` raised, when narrower, to the world width that projects to
/// [`STRIP_MIN_PX`] pixels at `deck_zoom` (`STRIP_MIN_PX / 2^deck_zoom` metres).
#[must_use]
pub fn clamp_strip_width_m(base_width_m: f64, deck_zoom: f64) -> f64 {
    base_width_m.max(STRIP_MIN_PX / 2.0_f64.powf(deck_zoom))
}

/// Endpoints, in world metres, of the oriented footprint's longer centre axis: the midpoints
/// of its two short edges; on a square footprint, the axis joining the midpoints of edges 3-0
/// and 1-2.
#[must_use]
pub fn obb_long_axis_endpoints(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
) -> [[f64; 2]; 2] {
    let c = prefab_catalog::footprint_lookups::obb_corners(x, y, half_x, half_y, rotation_deg);
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

/// Pier strip along the footprint's long axis in `fill_rgba`, as wide as its short side but
/// at most [`PIER_STRIP_MAX_WIDTH_M`], floored to [`STRIP_MIN_PX`] at `deck_zoom`.
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

/// Two railing strips parallel to the bridge's long axis, offset each side by half its short
/// side capped at [`BRIDGE_RAILING_RADIUS_M`], at fence width and colour.
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
