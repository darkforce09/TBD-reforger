//! Role: shot projection, panel keying, and the elevation chart geometry.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: panel identity follows WHERE the shot is, never its verdict, so a re-aim never retains a stale panel.

use super::super::capture::LosShot;
use super::super::terrain_verdict::LosVerdict;
use super::*;
use crate::spatial::los::terrain::sampler::ProfileSample;

/// A profile sample at an along-segment distance and a ground elevation.
fn s(dist_m: f64, elev_m: f64) -> ProfileSample {
    ProfileSample { dist_m, elev_m }
}

#[test]
fn project_shot_maps_endpoints_and_derives_verdict() {
    let shot = LosShot {
        obs_x: 0.0,
        obs_y: 0.0,
        obs_z: Some(100.0),
        tgt_x: 100.0,
        tgt_y: 0.0,
        tgt_z: Some(100.0),
    };
    // Flat profile → clear; identity-ish projector (scale 2, offset 5/7).
    let profile = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
    let proj = project_shot(&shot, &profile, 1.8, 1.8, |x, y| {
        (x * 2.0 + 5.0, y * 2.0 + 7.0)
    });
    assert!((proj.obs_px - 5.0).abs() < 1e-9 && (proj.obs_py - 7.0).abs() < 1e-9);
    assert!((proj.tgt_px - 205.0).abs() < 1e-9 && (proj.tgt_py - 7.0).abs() < 1e-9);
    assert_eq!(proj.verdict, LosVerdict::Clear);
    assert!(proj.block_px.is_none(), "clear → no blocking marker");
    assert!((proj.total_m - 100.0).abs() < 1e-9);
}

#[test]
fn project_shot_blocking_marker_on_the_line() {
    let shot = LosShot {
        obs_x: 0.0,
        obs_y: 0.0,
        obs_z: Some(100.0),
        tgt_x: 100.0,
        tgt_y: 0.0,
        tgt_z: Some(100.0),
    };
    // Ridge blocking at dist 50 (half-way) → marker at the line midpoint.
    let profile = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
    let proj = project_shot(&shot, &profile, 1.8, 1.8, |x, y| (x, y));
    assert!(proj.verdict.is_blocked());
    let (bx, by) = proj.block_px.expect("blocked → marker");
    assert!(
        (bx - 50.0).abs() < 1e-9 && (by - 0.0).abs() < 1e-9,
        "marker at the half-way pixel"
    );
}

/// Two DIFFERENT shots that share an identical VERDICT/label must get DIFFERENT keys — the
/// T-727 world-coordinate keying (a `<For>` keyed on text would retain a stale panel).
#[test]
fn shot_keys_are_world_coords_not_verdict() {
    let a = LosShot {
        obs_x: 0.0,
        obs_y: 0.0,
        obs_z: None,
        tgt_x: 100.0,
        tgt_y: 0.0,
        tgt_z: None,
    };
    let b = LosShot {
        obs_x: 500.0,
        obs_y: 500.0,
        obs_z: None,
        tgt_x: 600.0,
        tgt_y: 500.0,
        tgt_z: None,
    };
    let flat = [s(0.0, 10.0), s(100.0, 10.0)];
    let pa = project_shot(&a, &flat, 1.8, 1.8, |x, y| (x, y));
    let pb = project_shot(&b, &flat, 1.8, 1.8, |x, y| (x, y));
    // Same verdict (both clear over a flat profile) but distinct world-coord keys.
    assert_eq!(pa.verdict, pb.verdict);
    assert_ne!(
        pa.key, pb.key,
        "T-727: distinct shots get distinct world-coord keys"
    );
}

#[test]
fn world_key_quantises_and_distinguishes() {
    assert_eq!(
        world_key(10.02, 20.0, 30.0, 40.0),
        world_key(10.03, 20.0, 30.0, 40.0)
    );
    assert_ne!(
        world_key(10.0, 20.0, 30.0, 40.0),
        world_key(11.0, 20.0, 30.0, 40.0)
    );
}

// ── profile chart geometry (the inline panel) ─────────────────────────────────────────────────

#[test]
fn profile_chart_maps_into_the_box() {
    // A ramp profile 0..100 elev over 0..1000 dist, box 200×64.
    let prof = [s(0.0, 0.0), s(500.0, 50.0), s(1000.0, 100.0)];
    let chart = profile_chart(&prof, 1.8, 1.8, 200.0, 64.0);
    assert!(!chart.ground.is_empty(), "ground curve has points");
    assert!(!chart.line.is_empty(), "sight line has points");
    // The ground string has three "x,y" pairs; first x is 0, last x is the box width (200).
    let pts: Vec<&str> = chart.ground.split_whitespace().collect();
    assert_eq!(pts.len(), 3);
    assert!(pts[0].starts_with("0.0,"), "first ground x at box left");
    assert!(
        pts[2].starts_with("200.0,"),
        "last ground x at box right (200)"
    );
    // Ground rises left→right, so screen y DECREASES (inverted axis): y0 > y2.
    let y = |p: &str| p.split(',').nth(1).unwrap().parse::<f64>().unwrap();
    assert!(y(pts[0]) > y(pts[2]), "higher ground → smaller y (up)");
}

#[test]
fn profile_chart_empty_when_too_short() {
    assert_eq!(
        profile_chart(&[], 1.8, 1.8, 200.0, 64.0),
        ProfileChart::default()
    );
    assert_eq!(
        profile_chart(&[s(0.0, 1.0)], 1.8, 1.8, 200.0, 64.0),
        ProfileChart::default()
    );
}

#[test]
fn profile_chart_flat_profile_does_not_divide_by_zero() {
    // A perfectly flat profile (min == max ground, and eyes equal) still charts a mid line.
    let flat = [s(0.0, 50.0), s(100.0, 50.0)];
    let chart = profile_chart(&flat, 1.8, 1.8, 200.0, 64.0);
    assert!(!chart.ground.is_empty());
    // Every y is finite (no NaN from a zero span).
    for p in chart.ground.split_whitespace() {
        let y: f64 = p.split(',').nth(1).unwrap().parse().unwrap();
        assert!(y.is_finite(), "flat profile y must be finite, got {y}");
    }
}

#[test]
fn profile_chart_marks_blocking_point() {
    let prof = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
    let chart = profile_chart(&prof, 1.8, 1.8, 200.0, 64.0);
    let (bx, _by) = chart.block.expect("blocked profile marks the block");
    assert!(
        (bx - 100.0).abs() < 1e-6,
        "block x at the half-way column (100 of 200)"
    );
}
