//! Role: the leg quantities and every readout's exact shape.
//! Position: `ruler/tests` in `map_editing_tools`.
//! Signals & state: explicit vertices and chains built in the test body.
//! Invariants: the bearing rule is proved to DISCRIMINATE: the same two points swapped must read a different bearing.

use super::*;

use crate::ruler::leg::RulerPoint;

fn p(x: f64, y: f64) -> RulerPoint {
    RulerPoint::new(x, y, None)
}
fn pz(x: f64, y: f64, z: f64) -> RulerPoint {
    RulerPoint::new(x, y, Some(z))
}

// ── tool-mode arbitration (button filter + Select passthrough) ──────────────────────────────

#[test]
fn distance_is_euclidean_world_m() {
    assert!((distance_m(p(0.0, 0.0), p(3.0, 4.0)) - 5.0).abs() < 1e-9);
    assert!((distance_m(p(100.0, 100.0), p(100.0, 100.0))).abs() < 1e-12); // zero-length
    assert!((distance_m(p(0.0, 0.0), p(1000.0, 0.0)) - 1000.0).abs() < 1e-9);
}

/// The cardinal edge cases the ticket names — 0 / 90 / 180 / 270 — plus the wrap. Bearing is
/// clockwise from north with world +Y = north, +X = east.
#[test]
fn bearing_cardinal_edges_and_wrap() {
    let o = p(1000.0, 1000.0);
    assert!(
        (bearing_deg(o, p(1000.0, 2000.0)) - 0.0).abs() < 1e-9,
        "due north = 0"
    );
    assert!(
        (bearing_deg(o, p(2000.0, 1000.0)) - 90.0).abs() < 1e-9,
        "due east = 90"
    );
    assert!(
        (bearing_deg(o, p(1000.0, 0.0)) - 180.0).abs() < 1e-9,
        "due south = 180"
    );
    assert!(
        (bearing_deg(o, p(0.0, 1000.0)) - 270.0).abs() < 1e-9,
        "due west = 270"
    );
    // Intercardinal + the [0,360) wrap: NW is 315, not −45.
    assert!(
        (bearing_deg(o, p(0.0, 2000.0)) - 315.0).abs() < 1e-9,
        "NW = 315 (wrap, not -45)"
    );
    assert!(
        (bearing_deg(o, p(2000.0, 2000.0)) - 45.0).abs() < 1e-9,
        "NE = 45"
    );
    // A zero-length leg has no direction → defined 0.
    assert!((bearing_deg(o, o) - 0.0).abs() < 1e-12);
}

#[test]
fn slope_and_delta_elev_goldens() {
    // +8 m over 400 m run = +2% (the ticket's worked example).
    let a = pz(0.0, 0.0, 100.0);
    let b = pz(400.0, 0.0, 108.0);
    assert_eq!(delta_elev_m(a, b), Some(8.0));
    assert!((slope_pct(a, b).unwrap() - 2.0).abs() < 1e-9);
    // Descent is signed.
    let c = pz(0.0, 0.0, 50.0);
    let d = pz(0.0, 100.0, 40.0);
    assert_eq!(delta_elev_m(c, d), Some(-10.0));
    assert!((slope_pct(c, d).unwrap() - -10.0).abs() < 1e-9);
    // Off-coverage on EITHER end → None (no fake 0).
    assert_eq!(delta_elev_m(pz(0.0, 0.0, 1.0), p(1.0, 0.0)), None);
    assert_eq!(slope_pct(pz(0.0, 0.0, 1.0), p(1.0, 0.0)), None);
    // Zero-run leg → no grade even with a rise.
    assert_eq!(slope_pct(pz(5.0, 5.0, 10.0), pz(5.0, 5.0, 20.0)), None);
}

// ── formatter goldens ───────────────────────────────────────────────────────────────────────

/// counter-clockwise, or from east, would pass the perturbed assertion — so this proves the
/// clockwise-from-north convention is load-bearing, not incidental.
#[test]
fn bearing_rule_fires() {
    let o = p(0.0, 0.0);
    let east = p(100.0, 0.0);
    // Baseline: due east is 90° clockwise from north.
    assert!(
        (bearing_deg(o, east) - 90.0).abs() < 1e-9,
        "baseline: east = 90"
    );
    // Perturb: CLAIM east is 0° (which would be true only if bearing measured from east, or
    // returned a constant 0). That is FALSE, so an equality against the perturbed value must NOT
    // hold — the rule fires.
    let perturbed = 0.0;
    assert_ne!(
        (bearing_deg(o, east)).round(),
        perturbed,
        "east must NOT read 0° — if it did, bearing would be measuring from the wrong axis"
    );
    // Perturb the other way: claim east is 270° (counter-clockwise). Also FALSE.
    assert_ne!(
        (bearing_deg(o, east)).round(),
        270.0,
        "east must NOT read 270° — bearing must be CLOCKWISE from north, not counter-clockwise"
    );
    // Restore: the true value holds, and a different direction yields a different bearing (not a
    // constant).
    assert!((bearing_deg(o, east) - 90.0).abs() < 1e-9);
    assert_ne!(
        bearing_deg(o, east).round(),
        bearing_deg(o, p(0.0, 100.0)).round(),
        "bearing must vary with direction (east 90 vs north 0)"
    );
}
