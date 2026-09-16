//! Role: styling tests.
//! Position: `terrain/roads/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::roads::styling::*;

#[test]
fn polyline_width_midpoint_projection() {
    let width_m = 4.0_f64;
    let zoom = -2.0_f64;
    let expected_px = projected_width_px(width_m, zoom);
    assert!((expected_px - 1.0).abs() < 1e-12);

    let pts = [[0.0, 0.0], [100.0, 0.0]];
    let color = [1.0, 1.0, 1.0, 1.0];
    let strip = expand_polyline_strip(&pts, width_m, color);
    assert!(strip.len() >= 6);

    let left = strip[0].pos;
    let right = strip[1].pos;
    let world_width =
        (f64::from(left[0]) - f64::from(right[0])).hypot(f64::from(left[1]) - f64::from(right[1]));
    assert!(
        (world_width - width_m).abs() < 1e-5,
        "strip world width {world_width} != {width_m}"
    );
    let screen_px = world_width * 2.0_f64.powf(zoom);
    assert!(
        (screen_px - expected_px).abs() < 1e-6,
        "L9 fail: screen {screen_px} != expected {expected_px}"
    );
}

#[test]
fn casing_is_wider_by_factor() {
    let pts = [[0.0, 0.0], [50.0, 0.0]];
    let (casing, center) = compose_road_segment(&pts, 2.0, "road_paved", false);
    assert!(!casing.is_empty() && !center.is_empty());
    let c_w = (f64::from(casing[0].pos[1]) - f64::from(casing[1].pos[1])).abs();
    let n_w = (f64::from(center[0].pos[1]) - f64::from(center[1].pos[1])).abs();
    assert!((c_w - 2.0 * ROAD_CASING_FACTOR).abs() < 1e-4);
    assert!((n_w - 2.0).abs() < 1e-4);
}

#[test]
fn dashed_emits_more_than_solid_segment_count() {
    let pts = [[0.0, 0.0], [100.0, 0.0]];
    let color = [1.0, 1.0, 1.0, 1.0];
    let solid = expand_polyline_strip(&pts, 1.0, color);
    let dashed = expand_dashed_polyline_strip(&pts, 1.0, color, 8.0, 6.0);
    assert!(dashed.len() > solid.len());
}

#[test]
fn road_class_gates() {
    assert!(road_class_visible("highway_paved", -2.0));
    assert!(road_class_visible("road_dirt", -2.0));
    assert!(!road_class_visible("road_dirt", -2.1));
    assert!(!road_class_visible("path", 3.9));
    assert!(road_class_visible("path", 4.0));
}

#[test]
fn road_signature_matches_visibility_and_boundaries() {
    assert_eq!(road_class_signature(-7.0), 0);
    assert_eq!(road_class_signature(-6.0), 1);
    assert_eq!(road_class_signature(-2.0), 3);
    assert_eq!(road_class_signature(3.9), 3);
    assert_eq!(road_class_signature(4.0), 7);

    let classes = [
        "highway_paved",
        "road_paved",
        "runway",
        "road_dirt",
        "track",
        "path",
    ];
    let mut z = -8.0;
    while z <= 8.0 {
        let sig = road_class_signature(z);
        let sig2 = road_class_signature(z + 0.01);
        if sig == sig2 {
            for c in classes {
                assert_eq!(
                    road_class_visible(c, z),
                    road_class_visible(c, z + 0.01),
                    "class {c} visibility diverged within signature {sig} at z={z}"
                );
            }
        }
        z += 0.1;
    }
}

#[test]
fn runway_polish_width_at_zoom_zero() {
    let pts = [[0.0, 0.0], [200.0, 0.0]];
    let (_, center) = compose_runway_polish_segment(&pts, 4.0);
    assert!(!center.is_empty());
    let left = center[0].pos;
    let right = center[1].pos;
    let world_width =
        (f64::from(left[0]) - f64::from(right[0])).hypot(f64::from(left[1]) - f64::from(right[1]));
    assert!(
        (world_width - RUNWAY_POLISH_WIDTH_M).abs() < 0.05,
        "runway width {world_width} != {RUNWAY_POLISH_WIDTH_M}"
    );
    let screen_px = world_width * 2.0_f64.powf(0.0);
    assert!((screen_px - 20.0).abs() < 0.05);
}

#[test]
fn corner_join_covers_outer_bisector() {
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let width = 2.0_f64;
    let half = 1.0;
    let strip = expand_polyline_strip(&pts, width, [1.0, 1.0, 1.0, 1.0]);
    assert!(strip.len() > 12, "joined strip should exceed 2 bare quads");

    let sample = [10.0 + half * 0.5, -half * 0.5];
    assert!(
        point_in_any_tri(sample, &strip),
        "outer corner sample {sample:?} not covered — tear/gap at join"
    );
}

#[test]
fn end_cap_extends_past_endpoint() {
    let pts = [[0.0, 0.0], [10.0, 0.0]];
    let half = 1.0;
    let strip = expand_polyline_strip(&pts, 2.0, [1.0, 1.0, 1.0, 1.0]);

    let sample = [10.0 + half * 0.7, 0.0];
    assert!(
        point_in_any_tri(sample, &strip),
        "end cap sample {sample:?} not covered"
    );
}

fn point_in_any_tri(p: [f64; 2], strip: &[StripVertex]) -> bool {
    for tri in strip.chunks_exact(3) {
        let a = [f64::from(tri[0].pos[0]), f64::from(tri[0].pos[1])];
        let b = [f64::from(tri[1].pos[0]), f64::from(tri[1].pos[1])];
        let c = [f64::from(tri[2].pos[0]), f64::from(tri[2].pos[1])];
        if point_in_tri(p, a, b, c) {
            return true;
        }
    }
    false
}

fn point_in_tri(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let s = |u: [f64; 2], v: [f64; 2], w: [f64; 2]| {
        (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
    };
    let b1 = s(p, a, b) < 0.0;
    let b2 = s(p, b, c) < 0.0;
    let b3 = s(p, c, a) < 0.0;
    (b1 == b2) && (b2 == b3)
}
