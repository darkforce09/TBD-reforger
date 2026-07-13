//! T-152.4 — thin OBB strip composers for fence / pier / bridge-railing cartography.
//!
//! Reuses [`crate::geometry::polyline_strip::expand_polyline_strip`] (road strip math).

use crate::geometry::polyline_strip::{
    StripVertex, expand_polyline_strip, norm_rgba, pack_strip_verts,
};

/// Fence strip width in world meters (T-152.4 L3).
pub const FENCE_STRIP_WIDTH_M: f64 = 0.35;
/// Pier/dock long/short aspect threshold — below this, skip square fill (T-152.4 L4).
pub const PIER_ASPECT_MIN: f64 = 4.0;
/// Bridge centroid radius for railing fence association (T-152.4 L6 path A).
pub const BRIDGE_RAILING_RADIUS_M: f64 = 8.0;

/// Cartographic neutral fence/railing stroke `#8a8478` @ α0.85.
pub const FENCE_STRIP_RGBA: [u8; 4] = [0x8a, 0x84, 0x78, 217];

/// `max(hx,hy) / min(hx,hy)` with safe min clamp.
#[must_use]
pub fn obb_aspect_ratio(half_x: f64, half_y: f64) -> f64 {
    let a = half_x.abs().max(half_y.abs());
    let b = half_x.abs().min(half_y.abs()).max(1e-9);
    a / b
}

/// Two world-meter endpoints along the OBB long axis through `(x,y)`.
/// Rotation matches [`super::obb::obb_corners`] (0° = north, clockwise-positive).
#[must_use]
pub fn obb_long_axis_endpoints(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
) -> [[f64; 2]; 2] {
    let (long_half, extra_rot) = if half_x >= half_y {
        (half_x, 0.0)
    } else {
        (half_y, 90.0)
    };
    let rad = ((rotation_deg + extra_rot) * std::f64::consts::PI) / 180.0;
    let cos = rad.cos();
    let sin = rad.sin();
    // Local +x endpoint in obb_corners convention: rot(dx, 0) with dx = long_half.
    let forward = [cos * long_half, -sin * long_half];
    [
        [x - forward[0], y - forward[1]],
        [x + forward[0], y + forward[1]],
    ]
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

/// Fence prop strip at [`FENCE_STRIP_WIDTH_M`].
#[must_use]
pub fn compose_fence_strip(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
) -> Vec<StripVertex> {
    compose_obb_strip(
        x,
        y,
        half_x,
        half_y,
        rotation_deg,
        FENCE_STRIP_WIDTH_M,
        norm_rgba(FENCE_STRIP_RGBA),
    )
}

/// Pier/dock quay strip when aspect ≥ [`PIER_ASPECT_MIN`]; width = `min(hx,hy)×2`.
#[must_use]
pub fn compose_pier_strip(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    fill_rgba: [u8; 4],
) -> Option<Vec<StripVertex>> {
    if obb_aspect_ratio(half_x, half_y) < PIER_ASPECT_MIN {
        return None;
    }
    let width_m = half_x.min(half_y) * 2.0;
    Some(compose_obb_strip(
        x,
        y,
        half_x,
        half_y,
        rotation_deg,
        width_m,
        norm_rgba(fill_rgba),
    ))
}

/// Pack strip verts into flat `[x,y,r,g,b,a]…` for the render engine.
#[must_use]
pub fn pack_cartographic_strips(verts: &[StripVertex]) -> Vec<f32> {
    pack_strip_verts(verts)
}

/// Midpoint world width of a strip at a segment (Class R gate G6).
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
mod tests {
    use super::*;

    #[test]
    fn pier_aspect_gate() {
        assert!(obb_aspect_ratio(10.0, 1.5) >= PIER_ASPECT_MIN);
        assert!(obb_aspect_ratio(2.0, 2.0) < PIER_ASPECT_MIN);
        assert!(compose_pier_strip(0.0, 0.0, 10.0, 1.5, 0.0, FENCE_STRIP_RGBA).is_some());
        assert!(compose_pier_strip(0.0, 0.0, 2.0, 2.0, 0.0, FENCE_STRIP_RGBA).is_none());
    }

    #[test]
    fn fence_strip_width_midpoint() {
        let strip = compose_fence_strip(100.0, 200.0, 4.0, 0.5, 45.0);
        assert!(!strip.is_empty());
        let w = strip_world_width_at_midpoint(&strip).unwrap();
        assert!(
            (w - FENCE_STRIP_WIDTH_M).abs() < 0.01,
            "width {w} != {FENCE_STRIP_WIDTH_M}"
        );
    }

    #[test]
    fn fence_strip_vertex_count_positive() {
        let strip = compose_fence_strip(0.0, 0.0, 3.0, 0.4, 0.0);
        // Round caps + one quad minimum → well over 6 verts (2 tris).
        assert!(
            strip.len() >= 6,
            "expected triangle-list verts, got {}",
            strip.len()
        );
    }

    #[test]
    fn long_axis_length_is_twice_max_half() {
        let pts = obb_long_axis_endpoints(0.0, 0.0, 5.0, 1.0, 0.0);
        let len = (pts[1][0] - pts[0][0]).hypot(pts[1][1] - pts[0][1]);
        assert!((len - 10.0).abs() < 1e-9);
    }
}
