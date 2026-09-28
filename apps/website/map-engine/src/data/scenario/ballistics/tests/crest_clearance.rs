//! Tests of the crest clearance: a blocking and a non-blocking ridge under a solved flight, the
//! interpolation along the line of fire, the sample span and ordering, and malformed inputs.

use core::f64::consts::FRAC_PI_2;

use super::*;
use crate::data::scenario::ballistics::catalog::BallisticsCatalog;
use crate::data::scenario::ballistics::solver::solve_charge_elevation;
use crate::data::scenario::ballistics::wind::Wind;

const GUN_HEIGHT_M: f64 = 100.0;

fn sample(downrange_m: f64, height_m: f64) -> TerrainSample {
    TerrainSample {
        downrange_m,
        height_m,
    }
}

fn profile(samples: &[(f64, f64)]) -> TerrainProfile {
    TerrainProfile {
        samples: samples
            .iter()
            .map(|&(downrange_m, height_m)| sample(downrange_m, height_m))
            .collect(),
    }
}

/// A hand-made flight due north: up to 60 m at 200 m downrange, down to the muzzle height at
/// 300 m.
fn northbound_path() -> Vec<FlightSample> {
    [(0.0, 0.0), (100.0, 50.0), (200.0, 60.0), (300.0, 0.0)]
        .iter()
        .enumerate()
        .map(|(index, &(north, up))| FlightSample {
            time_s: index as f64,
            position_m: [0.0, north, up],
            velocity_m_s: [0.0, 100.0, 0.0],
        })
        .collect()
}

/// The `m252` firing `m821-he` at ring 2, 700 m on a bearing of 30°, 20 m uphill, in `wind`.
fn solved_ring_two(wind: Wind) -> (ChargeProblem, ChargeElevation) {
    let catalog =
        BallisticsCatalog::from_json_slice(include_bytes!("../catalog/tests/minimal_catalog.json"))
            .expect("the sample catalog decodes");
    let resolved = catalog
        .resolve_firing("m252", "m821-he", 2)
        .expect("the charge exists");
    let problem = ChargeProblem {
        flight_parameters: resolved.flight_parameters,
        muzzle_speed_m_s: resolved.muzzle_speed_m_s,
        elevation_limits_rad: resolved.weapon.elevation_limits_rad(),
        azimuth_rad: 30_f64.to_radians(),
        distance_m: 700.0,
        height_difference_m: 20.0,
        wind,
    };
    let solved = solve_charge_elevation(&problem).expect("ring 2 reaches 700 m");
    (problem, solved)
}

#[test]
fn a_ridge_below_the_trajectory_clears_and_one_above_it_blocks() {
    for wind in [
        Wind::CALM,
        Wind {
            speed_m_s: 7.0,
            from_deg: 300.0,
        },
    ] {
        let (problem, solved) = solved_ring_two(wind);
        let apex_height_m = GUN_HEIGHT_M + solved.apex_height_m;
        let low_ridge = profile(&[(0.0, GUN_HEIGHT_M - 1.0), (450.0, GUN_HEIGHT_M + 30.0)]);
        let clear = crest_clearance_of_charge(&problem, &solved, GUN_HEIGHT_M, &low_ridge)
            .expect("the low ridge is measured");
        assert!(!clear.is_blocked(), "{wind:?}: {clear:?}");
        assert_eq!(clear.first_blocking_downrange_m, None);
        assert_eq!(
            clear.min_clearance_m, 1.0,
            "the muzzle sample is the lowest clearance"
        );
        assert_eq!(clear.min_clearance_downrange_m, 0.0);

        let high_ridge = profile(&[
            (0.0, GUN_HEIGHT_M - 1.0),
            (450.0, apex_height_m + 50.0),
            (600.0, apex_height_m + 300.0),
        ]);
        let blocked = crest_clearance_of_charge(&problem, &solved, GUN_HEIGHT_M, &high_ridge)
            .expect("the high ridge is measured");
        assert!(blocked.is_blocked(), "{wind:?}: {blocked:?}");
        assert_eq!(blocked.first_blocking_downrange_m, Some(450.0));
        assert!(blocked.min_clearance_m < -299.0, "{blocked:?}");
        assert_eq!(blocked.min_clearance_downrange_m, 600.0);
    }
}

#[test]
fn the_muzzle_sample_clears_by_its_height_below_the_gun() {
    let (problem, solved) = solved_ring_two(Wind::CALM);
    let flat = profile(&[(0.0, GUN_HEIGHT_M - 2.0)]);
    let clearance =
        crest_clearance_of_charge(&problem, &solved, GUN_HEIGHT_M, &flat).expect("measured");
    assert_eq!(clearance.min_clearance_m, 2.0);
    assert_eq!(clearance.min_clearance_downrange_m, 0.0);
}

#[test]
fn clearance_interpolates_linearly_between_recorded_states() {
    let path = northbound_path();
    let terrain = profile(&[(150.0, GUN_HEIGHT_M + 54.0), (250.0, GUN_HEIGHT_M + 31.0)]);
    let clearance = crest_clearance(&path, 0.0, GUN_HEIGHT_M, &terrain).expect("measured");
    // At 150 m the flight is at 55 m, at 250 m at 30 m.
    assert!(
        (clearance.min_clearance_m - -1.0).abs() < 1e-12,
        "{clearance:?}"
    );
    assert_eq!(clearance.min_clearance_downrange_m, 250.0);
    assert_eq!(clearance.first_blocking_downrange_m, Some(250.0));
}

#[test]
fn the_first_blocking_sample_is_the_nearest_to_the_gun_whatever_the_input_order() {
    let path = northbound_path();
    let terrain = profile(&[
        (250.0, GUN_HEIGHT_M + 40.0),
        (50.0, GUN_HEIGHT_M + 30.0),
        (150.0, GUN_HEIGHT_M + 10.0),
    ]);
    let clearance = crest_clearance(&path, 0.0, GUN_HEIGHT_M, &terrain).expect("measured");
    assert_eq!(clearance.first_blocking_downrange_m, Some(50.0));
    assert_eq!(clearance.min_clearance_downrange_m, 250.0);
    assert!((clearance.min_clearance_m - -10.0).abs() < 1e-12);
}

#[test]
fn downrange_is_measured_along_the_line_azimuth() {
    let eastbound: Vec<FlightSample> = northbound_path()
        .into_iter()
        .map(|state| FlightSample {
            position_m: [state.position_m[1], 0.0, state.position_m[2]],
            ..state
        })
        .collect();
    let terrain = profile(&[(150.0, GUN_HEIGHT_M + 54.0)]);
    let clearance =
        crest_clearance(&eastbound, FRAC_PI_2, GUN_HEIGHT_M, &terrain).expect("measured");
    assert!(
        (clearance.min_clearance_m - 1.0).abs() < 1e-9,
        "{clearance:?}"
    );
}

#[test]
fn terrain_at_the_target_height_clears_by_exactly_zero_at_the_impact() {
    let path = northbound_path();
    let terrain = profile(&[(300.0, GUN_HEIGHT_M)]);
    let clearance = crest_clearance(&path, 0.0, GUN_HEIGHT_M, &terrain).expect("measured");
    assert_eq!(clearance.min_clearance_m, 0.0);
    assert!(!clearance.is_blocked());
}

#[test]
fn samples_beyond_the_impact_do_not_count() {
    let path = northbound_path();
    let terrain = profile(&[(100.0, GUN_HEIGHT_M), (350.0, GUN_HEIGHT_M + 500.0)]);
    let clearance = crest_clearance(&path, 0.0, GUN_HEIGHT_M, &terrain).expect("measured");
    assert!(!clearance.is_blocked());
    assert_eq!(clearance.min_clearance_downrange_m, 100.0);
    assert_eq!(
        crest_clearance(&path, 0.0, GUN_HEIGHT_M, &profile(&[(301.0, 0.0)])),
        Err(CrestClearanceError::NoSampleUnderFlight)
    );
    assert_eq!(
        crest_clearance(&path, 0.0, GUN_HEIGHT_M, &TerrainProfile::default()),
        Err(CrestClearanceError::NoSampleUnderFlight)
    );
}

#[test]
fn malformed_inputs_are_refused_without_panicking() {
    let path = northbound_path();
    let terrain = profile(&[(100.0, GUN_HEIGHT_M)]);
    let invalid = |result: Result<CrestClearance, CrestClearanceError>| match result {
        Err(CrestClearanceError::InvalidInput { parameter, .. }) => parameter,
        other => panic!("expected an invalid input, got {other:?}"),
    };
    assert_eq!(
        invalid(crest_clearance(&path, 0.0, f64::NAN, &terrain)),
        "gun_height_m"
    );
    assert_eq!(
        invalid(crest_clearance(
            &path,
            f64::INFINITY,
            GUN_HEIGHT_M,
            &terrain
        )),
        "line_azimuth_rad"
    );
    assert_eq!(
        invalid(crest_clearance(
            &path,
            0.0,
            GUN_HEIGHT_M,
            &profile(&[(-1.0, GUN_HEIGHT_M)])
        )),
        "profile.downrange_m"
    );
    assert_eq!(
        invalid(crest_clearance(
            &path,
            0.0,
            GUN_HEIGHT_M,
            &profile(&[(10.0, f64::NAN)])
        )),
        "profile.height_m"
    );
    let mut broken = path.clone();
    broken[1].position_m[2] = f64::NAN;
    assert_eq!(
        invalid(crest_clearance(&broken, 0.0, GUN_HEIGHT_M, &terrain)),
        "path.height_m"
    );
    assert_eq!(
        crest_clearance(&path[..1], 0.0, GUN_HEIGHT_M, &terrain),
        Err(CrestClearanceError::FlightNotRecorded)
    );
}

#[test]
fn a_clearance_round_trips_through_its_wire_form() {
    let clearance = CrestClearance {
        min_clearance_m: -3.5,
        min_clearance_downrange_m: 420.0,
        first_blocking_downrange_m: Some(400.0),
    };
    let wire = serde_json::to_value(clearance).expect("serialises");
    assert_eq!(
        wire,
        serde_json::json!({
            "min_clearance_m": -3.5,
            "min_clearance_downrange_m": 420.0,
            "first_blocking_downrange_m": 400.0
        })
    );
    let back: CrestClearance = serde_json::from_value(wire).expect("deserialises");
    assert_eq!(back, clearance);
}
