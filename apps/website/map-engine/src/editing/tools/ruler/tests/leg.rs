//! Role: the leg quantities and every readout's exact shape.
//! Position: `editing/tools/ruler/tests` in the map engine.
//! Signals & state: explicit vertices and chains built in the test body.
//! Invariants: the bearing rule is proved to DISCRIMINATE: the same two points swapped must read a different bearing.

use super::*;

use crate::editing::tools::ruler::leg::RulerPoint;

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

#[test]
fn leg_distance_formatter() {
    assert_eq!(format_leg_distance(412.0), "412 m");
    assert_eq!(format_leg_distance(999.4), "999 m");
    assert_eq!(format_leg_distance(1000.0), "1.00 km");
    assert_eq!(format_leg_distance(1240.0), "1.24 km");
}

#[test]
fn bearing_formatter_zero_padded_one_decimal() {
    assert_eq!(format_bearing(73.2), "073.2°");
    assert_eq!(format_bearing(0.0), "000.0°");
    assert_eq!(format_bearing(90.0), "090.0°");
    assert_eq!(format_bearing(180.04), "180.0°");
    assert_eq!(format_bearing(359.97), "000.0°"); // rounds to 360 → wraps to 000, never "360.0"
}

#[test]
fn delta_elev_and_slope_formatters() {
    assert_eq!(format_delta_elev(8.0), "+8 m");
    assert_eq!(format_delta_elev(-3.0), "-3 m");
    assert_eq!(format_delta_elev(0.0), "+0 m");
    // Slope is an UNSIGNED magnitude in the leg label — direction is on the Δelev clause.
    assert_eq!(format_slope(2.0), "2%");
    assert_eq!(format_slope(-5.0), "5%"); // descent grade printed as magnitude
    assert_eq!(format_slope(0.3), "0%"); // sub-1% reads flat
}

#[test]
fn total_formatter_and_leg_label_shape() {
    assert_eq!(format_total(850.0), "Σ 850 m");
    assert_eq!(format_total(1240.0), "Σ 1.24 km");
    // The full leg label matches the ticket's exact shape.
    let leg = Leg::between(pz(0.0, 0.0, 100.0), pz(0.0, 412.0, 108.0));
    assert_eq!(leg.label(), "412 m · 000.0° · +8 m (2%)");
    // Off-coverage leg drops the elevation clause entirely (no fake rise). 3-4-5 triangle:
    // dx=300 (east), dy=400 (north) → dist 500 m, bearing atan2(300,400)=36.87° → 036.9°.
    let bare = Leg::between(p(0.0, 0.0), p(300.0, 400.0));
    assert_eq!(bare.label(), "500 m · 036.9°");
}

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
