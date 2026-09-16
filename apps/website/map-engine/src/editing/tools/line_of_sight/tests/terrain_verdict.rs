//! Role: the sight-line math, the occlusion rule, and the readout formatters.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: the occlusion rule is proved to DISCRIMINATE, not merely to answer: the same profile raised and lowered must change the verdict.

use super::*;

use crate::spatial::los::terrain::sampler::ProfileSample;

fn s(dist_m: f64, elev_m: f64) -> ProfileSample {
    ProfileSample { dist_m, elev_m }
}

// ── sight-line interpolation ──────────────────────────────────────────────────────────────────

#[test]
fn sight_line_interpolates_eye_to_eye() {
    // Observer eye 10 m, target eye 20 m, over 100 m: midpoint is 15 m.
    assert!((sight_line_height(0.0, 100.0, 10.0, 20.0) - 10.0).abs() < 1e-9);
    assert!((sight_line_height(50.0, 100.0, 10.0, 20.0) - 15.0).abs() < 1e-9);
    assert!((sight_line_height(100.0, 100.0, 10.0, 20.0) - 20.0).abs() < 1e-9);
    // Zero-length segment → the observer eye (no distance to interpolate over).
    assert!((sight_line_height(0.0, 0.0, 7.0, 99.0) - 7.0).abs() < 1e-9);
}

// ── occlusion goldens (the ticket's required table) ───────────────────────────────────────────

/// Clear over flat ground: two endpoints at the same elevation. The line sits a full eye-height
/// above the ground at both ends, so nothing blocks — CLEAR.
#[test]
fn occlusion_clear_flat() {
    let prof = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
    assert_eq!(
        occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear
    );
}

/// Blocked by a synthetic ridge: flat ends, a tall spike in the middle that pokes above the
/// eye-to-eye line. The FIRST sample above the line is reported (the near side of the ridge).
#[test]
fn occlusion_blocked_by_ridge() {
    // Ground 100 m at the ends, a 150 m ridge at 400 m and 500 m; eyes at 101.8 m → the line is
    // ~101.8 m across (flat), so the 150 m ridge towers over it. First blocking sample at 400 m.
    let prof = [
        s(0.0, 100.0),
        s(200.0, 100.0),
        s(400.0, 150.0), // ridge near side — first above the line
        s(500.0, 150.0),
        s(800.0, 100.0),
    ];
    match occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M) {
        LosVerdict::Blocked {
            blocking_dist_m,
            blocking_elev_m,
        } => {
            assert!(
                (blocking_dist_m - 400.0).abs() < 1e-9,
                "first ridge sample reported"
            );
            assert!((blocking_elev_m - 150.0).abs() < 1e-9);
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

/// Grazing epsilon case: terrain that just TOUCHES the sight line reads CLEAR (strictly-above
/// rule + epsilon), while terrain a hair over the epsilon reads BLOCKED. Proves the boundary.
#[test]
fn occlusion_grazing_epsilon() {
    // Flat eyes at 100 + 1.8 = 101.8 m; a mid sample exactly at the line height (101.8) grazes.
    let graze = [s(0.0, 100.0), s(50.0, 101.8), s(100.0, 100.0)];
    assert_eq!(
        occlusion(&graze, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear,
        "terrain exactly at the sight line grazes → CLEAR (not blocked)"
    );
    // Within the epsilon above → still CLEAR (float-noise guard).
    let within = [
        s(0.0, 100.0),
        s(50.0, 101.8 + OCCLUSION_EPS_M * 0.5),
        s(100.0, 100.0),
    ];
    assert_eq!(
        occlusion(&within, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear,
        "terrain within epsilon of the line → CLEAR"
    );
    // A hair MORE than epsilon above → BLOCKED.
    let over = [
        s(0.0, 100.0),
        s(50.0, 101.8 + OCCLUSION_EPS_M * 2.0),
        s(100.0, 100.0),
    ];
    assert!(
        occlusion(&over, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M).is_blocked(),
        "terrain > line + epsilon → BLOCKED"
    );
}

/// Observer on a hill sees a valley: a high observer looks DOWN over intervening lower ground to
/// a low target — the down-sloping sight line clears everything. CLEAR.
#[test]
fn occlusion_observer_on_hill_sees_valley() {
    // Observer at 300 m ground (eye 301.8), target at 100 m ground (eye 101.8); the ground dips
    // to 90 m in between — well under the descending line. CLEAR.
    let prof = [
        s(0.0, 300.0),
        s(250.0, 150.0),
        s(500.0, 90.0),
        s(750.0, 95.0),
        s(1000.0, 100.0),
    ];
    assert_eq!(
        occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear,
        "a high observer's descending line clears the lower valley"
    );
}

/// A short profile (0/1 sample — entirely off DEM coverage) is Unknown, never a fake Clear.
#[test]
fn occlusion_off_coverage_is_unknown() {
    assert_eq!(occlusion(&[], 1.8, 1.8), LosVerdict::Unknown);
    assert_eq!(occlusion(&[s(0.0, 50.0)], 1.8, 1.8), LosVerdict::Unknown);
}

/// A two-sample flat profile can never self-block on its own endpoints: the line sits an eye
/// height above each ground endpoint, so a bare observer→target with nothing between is CLEAR.
#[test]
fn occlusion_endpoints_never_self_block() {
    let prof = [s(0.0, 42.0), s(500.0, 42.0)];
    assert_eq!(
        occlusion(&prof, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear
    );
    // Even a steep endpoint difference stays clear (the eyes are above both grounds).
    let steep = [s(0.0, 0.0), s(500.0, 300.0)];
    assert_eq!(
        occlusion(&steep, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear
    );
}

// ── formatter / verdict goldens ───────────────────────────────────────────────────────────────

#[test]
fn distance_formatter() {
    assert_eq!(format_distance(412.0), "412 m");
    assert_eq!(format_distance(999.4), "999 m");
    assert_eq!(format_distance(1000.0), "1.00 km");
    assert_eq!(format_distance(1240.0), "1.24 km");
}

#[test]
fn verdict_formatter() {
    assert_eq!(
        format_verdict(LosVerdict::Clear, 1240.0),
        "LoS clear · 1.24 km"
    );
    assert_eq!(
        format_verdict(
            LosVerdict::Blocked {
                blocking_dist_m: 412.0,
                blocking_elev_m: 150.0
            },
            800.0
        ),
        "LoS blocked at 412 m"
    );
    assert_eq!(format_verdict(LosVerdict::Unknown, 0.0), "LoS —");
    // is_clear / is_blocked helpers.
    assert!(LosVerdict::Clear.is_clear() && !LosVerdict::Clear.is_blocked());
    assert!(
        LosVerdict::Blocked {
            blocking_dist_m: 1.0,
            blocking_elev_m: 1.0
        }
        .is_blocked()
    );
    assert!(!LosVerdict::Unknown.is_clear() && !LosVerdict::Unknown.is_blocked());
}

// ── the occlusion rule genuinely discriminates ────────────────────────────────────────────────

/// A ridge that pokes above the sight line reads BLOCKED, and asserting it is CLEAR fails; lowering
/// that ridge below the line restores CLEAR. A rule that ignored the terrain (always Clear) would
/// pass the perturbed assertion — so this proves the strictly-above comparison is load-bearing.
#[test]
fn occlusion_rule_fires() {
    // Baseline: a 200 m ridge across a flat 100 m sight is BLOCKED.
    let blocked = [s(0.0, 100.0), s(50.0, 200.0), s(100.0, 100.0)];
    let v = occlusion(&blocked, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M);
    assert!(
        v.is_blocked(),
        "baseline: a 200 m ridge blocks a flat sight"
    );
    // Perturb: CLAIM it is clear. That is FALSE (the ridge towers over the line), so an equality
    // against Clear must NOT hold — the rule fires.
    assert_ne!(
        v,
        LosVerdict::Clear,
        "the ridge MUST block — if this were Clear the occlusion test would be ignoring terrain"
    );
    // Restore: drop the ridge below the sight line (to 100 m, flat) → CLEAR again.
    let cleared = [s(0.0, 100.0), s(50.0, 100.0), s(100.0, 100.0)];
    assert_eq!(
        occlusion(&cleared, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        LosVerdict::Clear,
        "lowering the ridge below the line restores a clear sight"
    );
    // And the verdict genuinely VARIES with the terrain (not a constant): blocked vs clear differ.
    assert_ne!(
        occlusion(&blocked, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        occlusion(&cleared, EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M),
        "the occlusion verdict must depend on the terrain profile"
    );
}
