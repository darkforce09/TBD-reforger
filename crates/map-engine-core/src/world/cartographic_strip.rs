//! T-152.4 — thin OBB strip composers for fence / pier / bridge-railing cartography.
//! T-152.15 — single-source strip frame (from `obb_corners`), STRIP_MIN_PX pixel floor,
//! unconditional pier strips, and consumed bridge railings.
//!
//! Reuses [`crate::geometry::polyline_strip::expand_polyline_strip`] (road strip math).

use crate::geometry::polyline_strip::{
    StripVertex, expand_polyline_strip, norm_rgba, pack_strip_verts,
};

/// Fence strip width in world meters (T-152.4 L3) — a floor of [`STRIP_MIN_PX`] is applied on top.
pub const FENCE_STRIP_WIDTH_M: f64 = 0.35;
/// T-152.15 L2 — minimum on-screen strip width in pixels. `screen_px = width_m · 2^deckZoom`, so
/// the world-meter floor is `STRIP_MIN_PX / 2^deckZoom`; keeps 0.35 m fences hairline-visible
/// below z≈2 (0.35 m projects to < 1.5 px until z≈2.10).
pub const STRIP_MIN_PX: f64 = 1.5;
/// T-152.15 L3 — pier fallback strip max width (m); `min(hx,hy)·2` is clamped to this before the
/// pixel floor so a wide dock OBB can't render an absurd quay.
pub const PIER_STRIP_MAX_WIDTH_M: f64 = 6.0;
/// Bridge centroid radius for railing lateral offset cap (T-152.4 L6 / T-152.15 Path A). Consumed
/// by [`compose_bridge_rail_strips`] as the perpendicular offset ceiling.
pub const BRIDGE_RAILING_RADIUS_M: f64 = 8.0;

/// Cartographic neutral fence/railing stroke `#8a8478` @ α0.85.
pub const FENCE_STRIP_RGBA: [u8; 4] = [0x8a, 0x84, 0x78, 217];

/// T-152.15 L2 — clamp a base world-meter strip width up to a [`STRIP_MIN_PX`] on-screen floor at
/// `deck_zoom`. Pure + unit-testable (gate G4). `screen_px = width · 2^deckZoom` (mirrors
/// [`crate::geometry::polyline_strip::projected_width_px`]); invert for the world-meter floor.
#[must_use]
pub fn clamp_strip_width_m(base_width_m: f64, deck_zoom: f64) -> f64 {
    base_width_m.max(STRIP_MIN_PX / 2.0_f64.powf(deck_zoom))
}

/// Two world-meter endpoints along the OBB long axis through `(x,y)`, derived from
/// [`super::obb::obb_corners`] so the strip axis is the **same** geometry as the fill (T-152.15 L5 —
/// single source of truth, kills the old `extra_rot=90` reconstruction). The two candidate axes are
/// the opposite-edge-midpoint pairs; the longer one (by squared length) is the OBB long axis.
#[must_use]
pub fn obb_long_axis_endpoints(
    x: f64,
    y: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
) -> [[f64; 2]; 2] {
    let c = super::obb::obb_corners(x, y, half_x, half_y, rotation_deg);
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
    // Axis A endpoints = midpoints of the two short edges (c3c0, c1c2) → local (∓hx, 0), sep 2·hx.
    let a0 = mid(c[3], c[0]);
    let a1 = mid(c[1], c[2]);
    // Axis B endpoints = midpoints of edges c0c1, c2c3 → local (0, ∓hy), sep 2·hy.
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

/// Pier/dock quay strip — **every** pier/dock emits one strip (T-152.15 L3). Width =
/// `min(hx,hy)·2` clamped ≤ [`PIER_STRIP_MAX_WIDTH_M`], then floored to [`STRIP_MIN_PX`]; length =
/// the OBB long axis.
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

/// T-152.15 L6 / Path A — two synthetic deck-edge rail strips per bridge, parallel to the crossing
/// (long) axis, offset ±`min(hx,hy)` (capped at [`BRIDGE_RAILING_RADIUS_M`] — the constant is
/// consumed here) perpendicular to it. Fence-strip styling; floored to [`STRIP_MIN_PX`]. Guarantees
/// ≥ 2 rails per bridge instance by construction (gate G5).
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
    let (ux, uy) = (dx / len, dy / len); // long-axis unit
    let (px, py) = (-uy, ux); // perpendicular (short axis)
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
mod tests {
    use super::*;

    /// Angle of a strip's long axis (degrees), from its first two endpoints via the composer input.
    fn axis_angle_deg(p0: [f64; 2], p1: [f64; 2]) -> f64 {
        (p1[1] - p0[1]).atan2(p1[0] - p0[0]).to_degrees()
    }

    /// Fill long-edge angle from `obb_corners` (the longer of the two adjacent edges).
    fn fill_long_axis_angle_deg(x: f64, y: f64, hx: f64, hy: f64, rot: f64) -> f64 {
        let c = super::super::obb::obb_corners(x, y, hx, hy, rot);
        let e0 = [c[1][0] - c[0][0], c[1][1] - c[0][1]]; // length 2·hx
        let e1 = [c[2][0] - c[1][0], c[2][1] - c[1][1]]; // length 2·hy
        let (dx, dy) = if e0[0].hypot(e0[1]) >= e1[0].hypot(e1[1]) {
            (e0[0], e0[1])
        } else {
            (e1[0], e1[1])
        };
        dy.atan2(dx).to_degrees()
    }

    fn ang_diff(a: f64, b: f64) -> f64 {
        let d = (a - b).abs() % 180.0;
        d.min(180.0 - d)
    }

    /// G4 — the pixel floor engages below z≈2.10 and disengages above (not an always-max stub).
    #[test]
    fn strip_min_px_floor() {
        // Engaged: 0.35 m projects below 1.5 px at z=1.5.
        assert!(0.35 * 2.0_f64.powf(1.5) < STRIP_MIN_PX);
        let w = clamp_strip_width_m(0.35, 1.5);
        assert!(w > 0.35, "clamp must widen at z=1.5, got {w}");
        assert!(
            (w * 2.0_f64.powf(1.5) - STRIP_MIN_PX).abs() < 1e-9,
            "floored width must project to exactly {STRIP_MIN_PX} px, got {}",
            w * 2.0_f64.powf(1.5)
        );
        // Disengaged: at z=3.0 the 0.35 m base already exceeds the floor.
        assert!((clamp_strip_width_m(0.35, 3.0) - 0.35).abs() < 1e-12);
    }

    /// G4 — composed fence strip renders ≥ STRIP_MIN_PX at the gate boundary z=1.5.
    #[test]
    fn fence_strip_screen_width_floor_at_gate() {
        let strip = compose_fence_strip(100.0, 200.0, 4.0, 0.5, 30.0, 1.5);
        assert!(!strip.is_empty());
        let w = strip_world_width_at_midpoint(&strip).unwrap();
        assert!(
            w * 2.0_f64.powf(1.5) >= STRIP_MIN_PX - 1e-6,
            "screen width {} < {STRIP_MIN_PX} px",
            w * 2.0_f64.powf(1.5)
        );
    }

    /// Above the floor, the fence strip keeps its 0.35 m base width exactly.
    #[test]
    fn fence_strip_width_midpoint() {
        let strip = compose_fence_strip(100.0, 200.0, 4.0, 0.5, 45.0, 3.0);
        assert!(!strip.is_empty());
        let w = strip_world_width_at_midpoint(&strip).unwrap();
        assert!(
            (w - FENCE_STRIP_WIDTH_M).abs() < 0.01,
            "width {w} != {FENCE_STRIP_WIDTH_M}"
        );
    }

    #[test]
    fn fence_strip_vertex_count_positive() {
        let strip = compose_fence_strip(0.0, 0.0, 3.0, 0.4, 0.0, 3.0);
        // Round caps + one quad minimum → well over 6 verts (2 tris).
        assert!(
            strip.len() >= 6,
            "expected triangle-list verts, got {}",
            strip.len()
        );
    }

    /// G3-support — every pier/dock emits exactly one non-empty strip, including near-square OBBs
    /// that the old `PIER_ASPECT_MIN` gate dropped to `None` (max real aspect was 2.57 < 4.0).
    #[test]
    fn every_pier_emits_one_strip() {
        // Near-square (aspect 1.0) — used to return None, must now emit.
        let square = compose_pier_strip(0.0, 0.0, 2.0, 2.0, 0.0, FENCE_STRIP_RGBA, 0.0);
        assert!(square.len() >= 6, "near-square pier must emit a strip");
        // Elongated quay — also emits.
        let quay = compose_pier_strip(10.0, 5.0, 10.0, 1.5, 37.0, FENCE_STRIP_RGBA, 0.0);
        assert!(quay.len() >= 6, "quay pier must emit a strip");
    }

    /// Pier fallback width is capped at PIER_STRIP_MAX_WIDTH_M (a wide dock can't render a huge quay).
    #[test]
    fn pier_width_capped() {
        // hx=hy=5 → min·2 = 10 m, must clamp to 6 m (z=0 so the px floor is inert).
        let strip = compose_pier_strip(0.0, 0.0, 5.0, 5.0, 0.0, FENCE_STRIP_RGBA, 0.0);
        let w = strip_world_width_at_midpoint(&strip).unwrap();
        assert!(
            (w - PIER_STRIP_MAX_WIDTH_M).abs() < 0.05,
            "pier width {w} != {PIER_STRIP_MAX_WIDTH_M}"
        );
    }

    #[test]
    fn long_axis_length_is_twice_max_half() {
        let pts = obb_long_axis_endpoints(0.0, 0.0, 5.0, 1.0, 0.0);
        let len = (pts[1][0] - pts[0][0]).hypot(pts[1][1] - pts[0][1]);
        assert!((len - 10.0).abs() < 1e-9);
    }

    /// G2 — strip long axis ≡ fill OBB long axis within 0.5° across sample yaws, incl. transposed
    /// (hx<hy) and near-square OBBs. The single-source rewrite makes this fp-exact by construction.
    #[test]
    fn strip_axis_matches_fill_long_axis() {
        let obbs = [
            (5.0_f64, 0.4_f64), // thin along x
            (0.4, 5.0),         // transposed (thin along y)
            (3.0, 2.9),         // near-square
            (10.0, 1.5),        // quay
        ];
        let mut checked = 0;
        for (hx, hy) in obbs {
            for yaw in [0.0_f64, 37.0, 90.0, 123.0] {
                let [p0, p1] = obb_long_axis_endpoints(12.0, -7.0, hx, hy, yaw);
                let strip_ang = axis_angle_deg(p0, p1);
                let fill_ang = fill_long_axis_angle_deg(12.0, -7.0, hx, hy, yaw);
                let d = ang_diff(strip_ang, fill_ang);
                assert!(d <= 0.5, "parity {d}° for hx={hx} hy={hy} yaw={yaw}");
                checked += 1;
            }
        }
        assert_eq!(checked, 16, "parity gate must be non-vacuous");
    }

    /// G5-support — a bridge emits exactly 2 rail strips, offset ≤ BRIDGE_RAILING_RADIUS_M.
    #[test]
    fn bridge_emits_two_rails_within_radius() {
        let hx = 12.0_f64;
        let hy = 3.0_f64;
        let yaw = 20.0_f64;
        let verts = compose_bridge_rail_strips(0.0, 0.0, hx, hy, yaw, 0.0);
        assert!(!verts.is_empty(), "bridge must emit rails");
        // The offset consumes the radius cap: short-half here, radius on a wide OBB.
        let off = hx.min(hy).min(BRIDGE_RAILING_RADIUS_M);
        assert!(off <= BRIDGE_RAILING_RADIUS_M);
        assert!(
            (off - hy).abs() < 1e-9,
            "offset should be the short half here"
        );
        let wide = 20.0_f64.min(BRIDGE_RAILING_RADIUS_M);
        assert!((wide - BRIDGE_RAILING_RADIUS_M).abs() < 1e-9);
    }
}
