//! Role: cartographic strip tests.
//! Position: `world/terrain/roads/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::roads::cartographic_strip::*;

fn axis_angle_deg(p0: [f64; 2], p1: [f64; 2]) -> f64 {
    (p1[1] - p0[1]).atan2(p1[0] - p0[0]).to_degrees()
}

fn fill_long_axis_angle_deg(x: f64, y: f64, hx: f64, hy: f64, rot: f64) -> f64 {
    let c = crate::world::environment::buildings::obb::obb_corners(x, y, hx, hy, rot);
    let e0 = [c[1][0] - c[0][0], c[1][1] - c[0][1]];
    let e1 = [c[2][0] - c[1][0], c[2][1] - c[1][1]];
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

#[test]
fn strip_min_px_floor() {
    assert!(0.35 * 2.0_f64.powf(1.5) < STRIP_MIN_PX);
    let w = clamp_strip_width_m(0.35, 1.5);
    assert!(w > 0.35, "clamp must widen at z=1.5, got {w}");
    assert!(
        (w * 2.0_f64.powf(1.5) - STRIP_MIN_PX).abs() < 1e-9,
        "floored width must project to exactly {STRIP_MIN_PX} px, got {}",
        w * 2.0_f64.powf(1.5)
    );

    assert!((clamp_strip_width_m(0.35, 3.0) - 0.35).abs() < 1e-12);
}

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

    assert!(
        strip.len() >= 6,
        "expected triangle-list verts, got {}",
        strip.len()
    );
}

#[test]
fn every_pier_emits_one_strip() {
    let square = compose_pier_strip(0.0, 0.0, 2.0, 2.0, 0.0, FENCE_STRIP_RGBA, 0.0);
    assert!(square.len() >= 6, "near-square pier must emit a strip");

    let quay = compose_pier_strip(10.0, 5.0, 10.0, 1.5, 37.0, FENCE_STRIP_RGBA, 0.0);
    assert!(quay.len() >= 6, "quay pier must emit a strip");
}

#[test]
fn pier_width_capped() {
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

#[test]
fn strip_axis_matches_fill_long_axis() {
    let obbs = [(5.0_f64, 0.4_f64), (0.4, 5.0), (3.0, 2.9), (10.0, 1.5)];
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

#[test]
fn bridge_emits_two_rails_within_radius() {
    let hx = 12.0_f64;
    let hy = 3.0_f64;
    let yaw = 20.0_f64;
    let verts = compose_bridge_rail_strips(0.0, 0.0, hx, hy, yaw, 0.0);
    assert!(!verts.is_empty(), "bridge must emit rails");

    let off = hx.min(hy).min(BRIDGE_RAILING_RADIUS_M);
    assert!(off <= BRIDGE_RAILING_RADIUS_M);
    assert!(
        (off - hy).abs() < 1e-9,
        "offset should be the short half here"
    );
    let wide = 20.0_f64.min(BRIDGE_RAILING_RADIUS_M);
    assert!((wide - BRIDGE_RAILING_RADIUS_M).abs() < 1e-9);
}
