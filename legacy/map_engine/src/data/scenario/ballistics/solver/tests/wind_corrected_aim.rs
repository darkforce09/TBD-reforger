//! Tests of the wind-corrected aim at the weapon's elevation limits: a crosswind target on
//! whose way a plain miss-following iteration meets an aim line refused by a range bound, but
//! which an aim point upwind of it reaches, solves onto the target; a crosswind target that no
//! aim point reaches keeps the range bound.

use crate::data::scenario::ballistics::angular_units::degrees_to_radians;
use crate::data::scenario::ballistics::catalog::{
    BallisticsCatalog, flight_parameters, muzzle_speed_m_s,
};
use crate::data::scenario::ballistics::flight_model::{Launch, PathRecording, fly_to_height};
use crate::data::scenario::ballistics::solver::charge_selection::{
    ChargeProblem, SolutionRefusal, solve_charge_elevation, solve_elevation_along,
};
use crate::data::scenario::ballistics::solver::wind_corrected_aim::{
    AIM_MISS_TOLERANCE_M, MAX_AIM_ITERATIONS,
};
use crate::data::scenario::ballistics::wind::Wind;

/// One mil of the 6400 convention, radians.
const ONE_MIL_6400_RAD: f64 = core::f64::consts::TAU / 6400.0;

/// How far above the lowest elevation the launch that makes the target lies, radians: the
/// oracle's widening of the weapon limits.
const LAUNCH_ABOVE_LOWEST_RAD: f64 = ONE_MIL_6400_RAD;

/// How far below the highest elevation the launch that makes the target lies, radians.
const LAUNCH_BELOW_HIGHEST_RAD: f64 = 0.1 * ONE_MIL_6400_RAD;

/// Horizontal distance within which the forward-flown aim lands on the target, metres.
const FORWARD_MISS_TOLERANCE_M: f64 = 0.05;

/// A strong crosswind from the east across a northward line of fire.
const CROSSWIND: Wind = Wind {
    speed_m_s: 12.0,
    from_deg: 90.0,
};

/// The hand-written test catalog: `m252` fires `m821-he` (70 m/s, ring 2 at ×2.2, drag
/// 0.0045 on 4.2 kg, wind influence 0.8).
fn drag_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("../../catalog/tests/minimal_catalog.json"))
        .expect("the sample catalog decodes")
}

/// The `m821-he` ring-2 problem whose target is where a northward launch at
/// `launch_elevation_rad` lands in [`CROSSWIND`], with the weapon laying only between
/// `elevation_limits_rad`.
fn crosswind_landing_problem(
    launch_elevation_rad: f64,
    elevation_limits_rad: (f64, f64),
) -> ChargeProblem {
    let catalog = drag_catalog();
    let weapon = catalog.weapon("m252").expect("the catalog has the m252");
    let shell = catalog.shell("m821-he").expect("the catalog has the m821");
    let charge = shell.charge(2).expect("the m821 has a ring-2 charge");
    let flight_parameters = flight_parameters(catalog.gravity_m_s2, shell);
    let muzzle_speed_m_s = muzzle_speed_m_s(weapon, shell, charge);
    let launch = Launch {
        muzzle_speed_m_s,
        elevation_rad: launch_elevation_rad,
        azimuth_rad: 0.0,
    };
    let landing = fly_to_height(
        &flight_parameters,
        &launch,
        &CROSSWIND,
        0.0,
        PathRecording::Discard,
    )
    .expect("the launch lands");
    let [east_m, north_m, _] = landing.impact_position_m;
    assert!(
        east_m < -1.0,
        "the crosswind from the east drifts the shell west, {east_m} m"
    );
    ChargeProblem {
        flight_parameters,
        muzzle_speed_m_s,
        elevation_limits_rad,
        azimuth_rad: libm::atan2(east_m, north_m).rem_euclid(core::f64::consts::TAU),
        distance_m: libm::hypot(east_m, north_m),
        height_difference_m: 0.0,
        wind: CROSSWIND,
    }
}

/// The first refusal a plain miss-following aim iteration meets on `problem`: the aim point
/// starts at the target and moves by each solved flight's miss, and a refused aim line ends it.
fn plain_iteration_refusal(problem: &ChargeProblem) -> Option<SolutionRefusal> {
    let target_m = [
        problem.distance_m * libm::sin(problem.azimuth_rad),
        problem.distance_m * libm::cos(problem.azimuth_rad),
    ];
    let mut aim_m = target_m;
    for _ in 0..MAX_AIM_ITERATIONS {
        let aim_azimuth_rad = libm::atan2(aim_m[0], aim_m[1]).rem_euclid(core::f64::consts::TAU);
        let aim_range_m = libm::hypot(aim_m[0], aim_m[1]);
        let flight = match solve_elevation_along(problem, aim_azimuth_rad, aim_range_m) {
            Ok((_, flight)) => flight,
            Err(refused) => return Some(refused.refusal),
        };
        let miss_m = [
            target_m[0] - flight.impact_position_m[0],
            target_m[1] - flight.impact_position_m[1],
        ];
        if libm::hypot(miss_m[0], miss_m[1]) < AIM_MISS_TOLERANCE_M {
            return None;
        }
        aim_m = [aim_m[0] + miss_m[0], aim_m[1] + miss_m[1]];
    }
    None
}

/// `problem` solves inside its elevation limits, within one mil of `launch_elevation_rad`, and
/// the forward-flown aim lands on the target.
fn assert_solves_onto_the_target(problem: &ChargeProblem, launch_elevation_rad: f64) {
    let solved = solve_charge_elevation(problem).expect("the corrected aim reaches the target");
    let (lowest_rad, highest_rad) = problem.elevation_limits_rad;
    assert!(
        (lowest_rad..=highest_rad).contains(&solved.elevation_rad),
        "elevation {} rad outside [{lowest_rad}, {highest_rad}]",
        solved.elevation_rad
    );
    let elevation_error_mils = (solved.elevation_rad - launch_elevation_rad) / ONE_MIL_6400_RAD;
    assert!(
        elevation_error_mils.abs() <= 1.0,
        "elevation {elevation_error_mils} mil from the launch"
    );
    assert!(
        solved.deflection_correction_rad > 0.0,
        "a crosswind from the east aims right of the target, {} rad",
        solved.deflection_correction_rad
    );
    let launch = Launch {
        muzzle_speed_m_s: problem.muzzle_speed_m_s,
        elevation_rad: solved.elevation_rad,
        azimuth_rad: solved.aim_azimuth_rad,
    };
    let flight = fly_to_height(
        &problem.flight_parameters,
        &launch,
        &problem.wind,
        problem.height_difference_m,
        PathRecording::Discard,
    )
    .expect("the solved aim lands");
    let target_m = [
        problem.distance_m * libm::sin(problem.azimuth_rad),
        problem.distance_m * libm::cos(problem.azimuth_rad),
    ];
    let miss_m = libm::hypot(
        flight.impact_position_m[0] - target_m[0],
        flight.impact_position_m[1] - target_m[1],
    );
    assert!(
        miss_m < FORWARD_MISS_TOLERANCE_M,
        "the forward-flown aim misses by {miss_m} m"
    );
}

#[test]
fn a_crosswind_target_at_the_lowest_elevation_solves_through_the_corrected_aim() {
    let launch_elevation_rad = degrees_to_radians(45.0);
    let problem = crosswind_landing_problem(
        launch_elevation_rad,
        (
            launch_elevation_rad - LAUNCH_ABOVE_LOWEST_RAD,
            degrees_to_radians(85.3),
        ),
    );
    assert_eq!(
        plain_iteration_refusal(&problem),
        Some(SolutionRefusal::OutOfRange)
    );
    assert_solves_onto_the_target(&problem, launch_elevation_rad);
}

#[test]
fn a_crosswind_target_at_the_highest_elevation_solves_through_the_corrected_aim() {
    let launch_elevation_rad = degrees_to_radians(80.0);
    let problem = crosswind_landing_problem(
        launch_elevation_rad,
        (
            degrees_to_radians(45.0),
            launch_elevation_rad + LAUNCH_BELOW_HIGHEST_RAD,
        ),
    );
    assert_eq!(
        plain_iteration_refusal(&problem),
        Some(SolutionRefusal::TooClose)
    );
    assert_solves_onto_the_target(&problem, launch_elevation_rad);
}

#[test]
fn a_crosswind_target_beyond_every_aim_point_stays_out_of_range() {
    let launch_elevation_rad = degrees_to_radians(45.0);
    let mut problem = crosswind_landing_problem(
        launch_elevation_rad,
        (
            launch_elevation_rad - LAUNCH_ABOVE_LOWEST_RAD,
            degrees_to_radians(85.3),
        ),
    );
    problem.distance_m *= 1.05;
    assert_eq!(
        solve_charge_elevation(&problem),
        Err(SolutionRefusal::OutOfRange)
    );
}

#[test]
fn a_crosswind_target_nearer_than_every_aim_point_stays_too_close() {
    let launch_elevation_rad = degrees_to_radians(80.0);
    let mut problem = crosswind_landing_problem(
        launch_elevation_rad,
        (
            degrees_to_radians(45.0),
            launch_elevation_rad + LAUNCH_BELOW_HIGHEST_RAD,
        ),
    );
    problem.distance_m *= 0.8;
    assert_eq!(
        solve_charge_elevation(&problem),
        Err(SolutionRefusal::TooClose)
    );
}
