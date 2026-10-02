//! Tests of the impact dispersion: deflection symmetry, growth of the range probable error with
//! range, the zero spread, finite-difference stability, the speed-spread unit, the transverse
//! secant, the wind-corrected aim and the agreement of the double-precision flight the
//! derivatives use with the engine's single-precision flight.

use super::*;
use crate::data::scenario::ballistics::catalog::BallisticsCatalog;
use crate::data::scenario::ballistics::flight_model::fly_to_height;
use crate::data::scenario::ballistics::solver::{
    ChargeSolution, FireSolutionRequest, MapPosition, solve_fire_solution,
};
use crate::data::scenario::ballistics::wind::Wind;

/// The game's shared mortar values: DispersionDiameter 1 m at DispersionRange 48 m,
/// DispersionMultiplier 1, InitSpeedVariation 3 m/s, range-card standard dispersion 15 m.
const GAME_SPREAD: DispersionParameters = DispersionParameters {
    dispersion_diameter_m: 1.0,
    dispersion_multiplier: 1.0,
    dispersion_range_m: 48.0,
    init_speed_variation_m_s: 3.0,
    standard_dispersion_m: 15.0,
};

fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("../catalog/tests/minimal_catalog.json"))
        .expect("the sample catalog decodes")
}

fn at(x_m: f64, y_m: f64, height_m: f64) -> MapPosition {
    MapPosition { x_m, y_m, height_m }
}

fn request(target: MapPosition, wind: Wind) -> FireSolutionRequest<'static> {
    FireSolutionRequest {
        weapon_id: "m252",
        shell_id: "m821-he",
        gun: at(0.0, 0.0, 0.0),
        target,
        wind,
    }
}

/// The solved row of `rings` rings, or `None` when that charge refuses.
fn solved_charge(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    rings: u32,
) -> Option<ChargeSolution> {
    let solution = solve_fire_solution(catalog, request).expect("the request is well formed");
    solution
        .charges
        .into_iter()
        .find(|charge| charge.rings == rings && charge.solves())
}

/// The dispersion problem of a solved row with the game's shared spread values.
fn game_problem(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    charge: &ChargeSolution,
) -> DispersionProblem {
    let firing = catalog
        .resolve_firing(request.weapon_id, request.shell_id, charge.rings)
        .expect("the charge exists");
    DispersionProblem {
        flight_parameters: firing.flight_parameters,
        muzzle_speed_m_s: firing.muzzle_speed_m_s,
        elevation_rad: degrees_to_radians(charge.elevation_deg.expect("solved")),
        aim_azimuth_rad: degrees_to_radians(charge.aim_azimuth_deg.expect("solved")),
        height_difference_m: request.target.height_m - request.gun.height_m,
        wind: request.wind,
        spread: GAME_SPREAD,
    }
}

fn relative_difference(left: f64, right: f64) -> f64 {
    (left - right).abs() / left.abs().max(right.abs())
}

#[test]
fn ellipse_is_symmetric_in_deflection_in_calm_air() {
    let catalog = catalog();
    let request = request(at(300.0, 400.0, 12.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 500 m");
    let problem = game_problem(&catalog, &request, &charge);
    let analysis = impact_dispersion(&problem).expect("the dispersion computes");

    let scale = (analysis.range_variance_m2 * analysis.deflection_variance_m2).sqrt();
    assert!(
        analysis.range_deflection_covariance_m2.abs() <= 1e-6 * scale,
        "range-deflection covariance {} against scale {scale}",
        analysis.range_deflection_covariance_m2
    );
    let axis_offset_deg = (analysis.dispersion.ellipse_orientation_deg
        - radians_to_degrees(problem.aim_azimuth_rad))
    .rem_euclid(90.0);
    assert!(
        axis_offset_deg.min(90.0 - axis_offset_deg) < 1e-6,
        "the ellipse axes lie along and across the line of fire (offset {axis_offset_deg}°)"
    );

    let transverse_rad = 1e-3 / libm::cos(problem.elevation_rad);
    let [left, right] = [-transverse_rad, transverse_rad].map(|offset| {
        let impact = horizontal_impact_m(
            &problem,
            problem.elevation_rad,
            problem.aim_azimuth_rad + offset,
            problem.muzzle_speed_m_s,
        )
        .expect("the perturbed flight lands");
        range_and_deflection(impact, analysis.range_axis_bearing_rad)
    });
    assert!(
        (left[0] - right[0]).abs() < 1e-6,
        "mirrored impacts share the range: {left:?} {right:?}"
    );
    assert!(
        (left[1] + right[1]).abs() < 1e-6 && right[1] > 0.0,
        "mirrored impacts have opposite deflections: {left:?} {right:?}"
    );
}

#[test]
fn range_probable_error_grows_with_range_for_a_fixed_charge() {
    // Rings 0 and 1: the total range PE grows across the whole solvable window. The speed
    // spread's share grows for every charge; ring 2's steep end is dominated by the pitch
    // share, which shrinks as the elevation falls toward the maximum-range angle, so its total
    // is pinned only through the speed share.
    let catalog = catalog();
    for rings in [0, 1, 2] {
        let mut previous: Option<(f64, f64, f64)> = None;
        let mut solved = 0;
        for step in 1..=60 {
            let distance_m = 25.0 * f64::from(step);
            let request = request(at(0.0, distance_m, 0.0), Wind::CALM);
            let Some(charge) = solved_charge(&catalog, &request, rings) else {
                continue;
            };
            solved += 1;
            let problem = game_problem(&catalog, &request, &charge);
            let total_m = impact_dispersion(&problem)
                .expect("the dispersion computes")
                .dispersion
                .range_probable_error_m;
            let mut speed_only = problem;
            speed_only.spread.dispersion_diameter_m = 0.0;
            let speed_share_m = impact_dispersion(&speed_only)
                .expect("the dispersion computes")
                .dispersion
                .range_probable_error_m;
            if let Some((previous_distance_m, previous_total_m, previous_speed_m)) = previous {
                assert!(
                    speed_share_m > previous_speed_m,
                    "ring {rings}: speed-share range PE {speed_share_m} m at {distance_m} m is \
                     not above {previous_speed_m} m at {previous_distance_m} m"
                );
                assert!(
                    rings == 2 || total_m > previous_total_m,
                    "ring {rings}: range PE {total_m} m at {distance_m} m is not above \
                     {previous_total_m} m at {previous_distance_m} m"
                );
            }
            previous = Some((distance_m, total_m, speed_share_m));
        }
        assert!(
            solved >= 4,
            "ring {rings} solves only {solved} lattice distances"
        );
    }
}

#[test]
fn zero_dispersion_parameters_give_a_zero_ellipse() {
    let catalog = catalog();
    let request = request(at(0.0, 450.0, 0.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 450 m");
    let mut problem = game_problem(&catalog, &request, &charge);
    problem.spread = DispersionParameters {
        dispersion_diameter_m: 0.0,
        dispersion_multiplier: 1.0,
        dispersion_range_m: 48.0,
        init_speed_variation_m_s: 0.0,
        standard_dispersion_m: 15.0,
    };
    let dispersion = impact_dispersion(&problem)
        .expect("the dispersion computes")
        .dispersion;
    assert_eq!(dispersion.range_probable_error_m, 0.0);
    assert_eq!(dispersion.deflection_probable_error_m, 0.0);
    assert_eq!(dispersion.ellipse_semi_major_m, 0.0);
    assert_eq!(dispersion.ellipse_semi_minor_m, 0.0);
    assert_eq!(dispersion.standard_dispersion_m, 15.0);
    assert!(!dispersion.verified_in_engine);

    problem.spread.dispersion_diameter_m = 1.0;
    problem.spread.dispersion_multiplier = 0.0;
    let multiplied_away = impact_dispersion(&problem).expect("the dispersion computes");
    assert_eq!(multiplied_away.dispersion.ellipse_semi_major_m, 0.0);
}

#[test]
fn halving_the_finite_difference_steps_agrees_within_one_percent() {
    let catalog = catalog();
    let half_steps = FiniteDifferenceSteps {
        angle_rad: FiniteDifferenceSteps::DEFAULT.angle_rad / 2.0,
        speed_fraction: FiniteDifferenceSteps::DEFAULT.speed_fraction / 2.0,
    };
    let mut compared = 0;
    for rings in [0, 1, 2] {
        for (target, wind) in [
            (at(0.0, 250.0, 0.0), Wind::CALM),
            (at(300.0, 400.0, 20.0), Wind::CALM),
            (at(-500.0, 350.0, -15.0), Wind::CALM),
            (
                at(420.0, -260.0, 5.0),
                Wind {
                    speed_m_s: 6.0,
                    from_deg: 250.0,
                },
            ),
        ] {
            let request = request(target, wind);
            let Some(charge) = solved_charge(&catalog, &request, rings) else {
                continue;
            };
            let problem = game_problem(&catalog, &request, &charge);
            let full = impact_dispersion(&problem).expect("default steps compute");
            let half = impact_dispersion_with_steps(&problem, half_steps).expect("half steps");
            for (name, left, right) in [
                (
                    "range PE",
                    full.dispersion.range_probable_error_m,
                    half.dispersion.range_probable_error_m,
                ),
                (
                    "deflection PE",
                    full.dispersion.deflection_probable_error_m,
                    half.dispersion.deflection_probable_error_m,
                ),
                (
                    "semi-major",
                    full.dispersion.ellipse_semi_major_m,
                    half.dispersion.ellipse_semi_major_m,
                ),
                (
                    "semi-minor",
                    full.dispersion.ellipse_semi_minor_m,
                    half.dispersion.ellipse_semi_minor_m,
                ),
            ] {
                assert!(
                    relative_difference(left, right) < 0.01,
                    "ring {rings} target {target:?}: {name} {left} (h) vs {right} (h/2)"
                );
            }
            compared += 1;
        }
    }
    assert!(compared >= 6, "only {compared} cases solved");
}

#[test]
fn init_speed_variation_is_a_spread_in_metres_per_second() {
    let catalog = catalog();
    let request = request(at(0.0, 500.0, 0.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 500 m");
    let mut problem = game_problem(&catalog, &request, &charge);
    problem.spread.dispersion_diameter_m = 0.0;
    let analysis = impact_dispersion(&problem).expect("the dispersion computes");

    assert!((problem.spread.speed_sigma_m_s() - 3.0 / 3.0_f64.sqrt()).abs() < 1e-15);
    let range_at = |speed_m_s: f64| {
        horizontal_impact_m(
            &problem,
            problem.elevation_rad,
            problem.aim_azimuth_rad,
            speed_m_s,
        )
        .expect("the flight lands")[1]
    };
    let per_metre_per_second =
        (range_at(problem.muzzle_speed_m_s + 0.5) - range_at(problem.muzzle_speed_m_s - 0.5)) / 1.0;
    let expected_m = PROBABLE_ERROR_PER_SIGMA * (3.0 / 3.0_f64.sqrt()) * per_metre_per_second.abs();
    assert!(
        relative_difference(analysis.dispersion.range_probable_error_m, expected_m) < 1e-3,
        "range PE {} m against ±3 m/s read as metres per second {expected_m} m",
        analysis.dispersion.range_probable_error_m
    );
    assert_eq!(analysis.dispersion.deflection_probable_error_m, 0.0);
}

#[test]
fn transverse_spread_lays_the_azimuth_off_by_the_secant_of_the_elevation() {
    let catalog = catalog();
    let request = request(at(0.0, 400.0, 0.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 400 m");
    let mut problem = game_problem(&catalog, &request, &charge);
    problem.spread.init_speed_variation_m_s = 0.0;
    let analysis = impact_dispersion(&problem).expect("the dispersion computes");

    let angular_sigma_rad = 0.5 / 48.0 / 2.0;
    assert!((problem.spread.angular_sigma_rad() - angular_sigma_rad).abs() < 1e-15);
    let expected_m = PROBABLE_ERROR_PER_SIGMA * angular_sigma_rad * analysis.nominal_impact_m[1]
        / libm::cos(problem.elevation_rad);
    assert!(
        relative_difference(analysis.dispersion.deflection_probable_error_m, expected_m) < 1e-4,
        "deflection PE {} m against R·σ/cos θ {expected_m} m",
        analysis.dispersion.deflection_probable_error_m
    );
}

#[test]
fn wind_corrected_aim_centres_the_ellipse_on_the_target() {
    let catalog = catalog();
    let target = at(350.0, 250.0, 8.0);
    let request = request(
        target,
        Wind {
            speed_m_s: 8.0,
            from_deg: 300.0,
        },
    );
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves in the wind");
    assert!(charge.deflection_correction_mils.expect("solved").abs() > 1.0);
    let analysis = charge_dispersion(&catalog, &request, &charge).expect("the dispersion computes");

    let miss_m = libm::hypot(
        analysis.nominal_impact_m[0] - target.x_m,
        analysis.nominal_impact_m[1] - target.y_m,
    );
    assert!(
        miss_m < 0.05,
        "the nominal impact misses the target by {miss_m} m"
    );
    let target_bearing_rad = libm::atan2(target.x_m, target.y_m);
    assert!((analysis.range_axis_bearing_rad - target_bearing_rad).abs() < 1e-3);
    assert!(analysis.dispersion.range_probable_error_m > 0.0);
}

#[test]
fn charge_dispersion_uses_the_catalog_spread_and_refuses_unsolved_rows() {
    let catalog = catalog();
    let request = request(at(0.0, 450.0, 0.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 450 m");
    let analysis = charge_dispersion(&catalog, &request, &charge).expect("the dispersion computes");
    assert_eq!(analysis.dispersion.standard_dispersion_m, 12.0);
    assert!(!analysis.dispersion.verified_in_engine);

    let refused = ChargeSolution::from_result(
        1,
        Err(SolutionRefusal::OutOfRange),
        crate::data::scenario::ballistics::angular_units::MilsConvention::MILS_6400,
    );
    assert_eq!(
        charge_dispersion(&catalog, &request, &refused),
        Err(DispersionError::ChargeDidNotSolve {
            rings: 1,
            refusal: Some(SolutionRefusal::OutOfRange),
        })
    );
    let unknown_ring = ChargeSolution { rings: 7, ..charge };
    assert!(matches!(
        charge_dispersion(&catalog, &request, &unknown_ring),
        Err(DispersionError::Lookup(CatalogLookupError::UnknownRing {
            rings: 7,
            ..
        }))
    ));
}

#[test]
fn malformed_inputs_are_typed_errors() {
    let catalog = catalog();
    let request = request(at(0.0, 450.0, 0.0), Wind::CALM);
    let charge = solved_charge(&catalog, &request, 1).expect("ring 1 solves at 450 m");
    let problem = game_problem(&catalog, &request, &charge);
    let refused_parameter = |problem: DispersionProblem| match impact_dispersion(&problem) {
        Err(DispersionError::InvalidInput { parameter, .. }) => parameter,
        other => panic!("expected an invalid input, got {other:?}"),
    };

    let mut case = problem;
    case.spread.dispersion_range_m = 0.0;
    assert_eq!(refused_parameter(case), "dispersion_range_m");
    let mut case = problem;
    case.spread.init_speed_variation_m_s = f64::NAN;
    assert_eq!(refused_parameter(case), "init_speed_variation_m_s");
    let mut case = problem;
    case.elevation_rad = core::f64::consts::FRAC_PI_2;
    assert_eq!(refused_parameter(case), "elevation_rad");
    let mut case = problem;
    case.height_difference_m = f64::INFINITY;
    assert_eq!(refused_parameter(case), "height_difference_m");
    let mut case = problem;
    case.muzzle_speed_m_s = 0.0;
    assert!(matches!(
        impact_dispersion(&case),
        Err(DispersionError::Flight(FlightError::InvalidInput { .. }))
    ));
    let zero_step = FiniteDifferenceSteps {
        angle_rad: 0.0,
        speed_fraction: 1e-3,
    };
    assert!(matches!(
        impact_dispersion_with_steps(&problem, zero_step),
        Err(DispersionError::InvalidInput {
            parameter: "angle_rad",
            ..
        })
    ));
}

#[test]
fn dispersion_serialises_the_contract_fields() {
    let dispersion = ImpactDispersion {
        range_probable_error_m: 4.0,
        deflection_probable_error_m: 2.0,
        ellipse_semi_major_m: 7.0,
        ellipse_semi_minor_m: 3.5,
        ellipse_orientation_deg: 36.0,
        standard_dispersion_m: 15.0,
        verified_in_engine: false,
    };
    let value = serde_json::to_value(dispersion).expect("serialises");
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "deflection_probable_error_m",
            "ellipse_orientation_deg",
            "ellipse_semi_major_m",
            "ellipse_semi_minor_m",
            "range_probable_error_m",
            "standard_dispersion_m",
            "verified_in_engine",
        ]
    );
    assert_eq!(value["verified_in_engine"], serde_json::Value::Bool(false));
}

#[test]
fn double_precision_flight_lands_within_two_centimetres_of_the_engine_flight() {
    let catalog = catalog();
    let winds = [
        Wind::CALM,
        Wind {
            speed_m_s: 8.0,
            from_deg: 300.0,
        },
    ];
    let mut compared = 0;
    let mut largest_m: f64 = 0.0;
    for wind in winds {
        for rings in [0, 1, 2] {
            for step in 1..=16 {
                let distance_m = 100.0 * f64::from(step);
                let request = request(at(0.6 * distance_m, 0.8 * distance_m, 10.0), wind);
                let Some(charge) = solved_charge(&catalog, &request, rings) else {
                    continue;
                };
                let problem = game_problem(&catalog, &request, &charge);
                let double_m = horizontal_impact_m(
                    &problem,
                    problem.elevation_rad,
                    problem.aim_azimuth_rad,
                    problem.muzzle_speed_m_s,
                )
                .expect("the double-precision flight lands");
                let single = fly_to_height(
                    &problem.flight_parameters,
                    &Launch {
                        muzzle_speed_m_s: problem.muzzle_speed_m_s,
                        elevation_rad: problem.elevation_rad,
                        azimuth_rad: problem.aim_azimuth_rad,
                    },
                    &problem.wind,
                    problem.height_difference_m,
                    PathRecording::Discard,
                )
                .expect("the engine flight lands");
                let separation_m = libm::hypot(
                    double_m[0] - single.impact_position_m[0],
                    double_m[1] - single.impact_position_m[1],
                );
                assert!(
                    separation_m <= 0.02,
                    "ring {rings} at {distance_m} m in {wind:?}: the f64 and f32 points of fall \
                     lie {separation_m} m apart"
                );
                largest_m = largest_m.max(separation_m);
                compared += 1;
            }
        }
    }
    assert!(compared >= 20, "only {compared} shots solve on the lattice");
    assert!(
        largest_m > 0.0,
        "the two precisions agree to the bit, so the comparison proves nothing"
    );
}

#[test]
fn dispersion_read_refuses_a_claim_of_in_engine_verification() {
    let wire = |verified: bool| {
        serde_json::json!({
            "range_probable_error_m": 4.0,
            "deflection_probable_error_m": 2.0,
            "ellipse_semi_major_m": 7.0,
            "ellipse_semi_minor_m": 3.5,
            "ellipse_orientation_deg": 36.0,
            "standard_dispersion_m": 15.0,
            "verified_in_engine": verified,
        })
    };
    let read: ImpactDispersion =
        serde_json::from_value(wire(false)).expect("the contract's const false reads");
    assert!(!read.verified_in_engine);
    let claim = serde_json::from_value::<ImpactDispersion>(wire(true))
        .expect_err("a dispersion claiming in-engine verification is refused");
    assert!(
        claim.to_string().contains("verified_in_engine"),
        "the refusal names the field: {claim}"
    );
}
