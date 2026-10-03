//! Tests of the firing solver: the vacuum limit, the refusals, the iteration bound, the charge
//! recommendation, malformed inputs, the angle conventions and the lifetime-capped elevation
//! bracket; the wind-corrected aim cases are in `solver_wind_aim.rs`.

use std::sync::LazyLock;

use ballistics_model::ids::{ShellId, WeaponId};
use core::f64::consts::FRAC_PI_4;

use super::elevation_search::{
    GOLDEN_SECTION_ITERATIONS, RootSearchError, RootSearchLimits, find_bracketed_root,
    maximise_by_golden_section,
};
use super::*;
use ballistics_model::angular_units::degrees_to_radians;
use ballistics_model::catalog::{BallisticsCatalog, CatalogLookupError};
use ballistics_model::flight_model::{
    DEFAULT_INTEGRATION_STEP_S, Launch, PathRecording, fly_to_height,
};
use ballistics_model::wind::{Wind, WindError};

/// The test catalog's M252 launcher, borrowed by the `'static` requests.
static M252: LazyLock<WeaponId> = LazyLock::new(|| WeaponId::new("m252"));
/// The test catalog's M821 high-explosive shell, borrowed by the `'static` requests.
static M821_HE: LazyLock<ShellId> = LazyLock::new(|| ShellId::new("m821-he"));

const GRAVITY_M_S2: f64 = 9.81;

/// The hand-written test catalog: `m252` (6,400 mils, 45°–85.3°) fires `m821-he` (70 m/s,
/// rings 0/1/2 at ×1.0/1.6/2.2); `2b14` (6,000 mils, 45°–85°, muzzle ×0.95) fires `o-832-he`
/// (72 m/s, rings 0/4 at ×1.0/2.5).
fn drag_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/ballistics/minimal_catalog.json"
    )))
    .expect("the sample catalog decodes")
}

/// The test catalog with every shell's air drag removed, so flights are analytic.
fn vacuum_catalog() -> BallisticsCatalog {
    let mut catalog = drag_catalog();
    for shell in &mut catalog.shells {
        shell.air_drag = 0.0;
    }
    catalog
}

fn at(x_m: f64, y_m: f64, height_m: f64) -> MapPosition {
    MapPosition { x_m, y_m, height_m }
}

fn m252_request(gun: MapPosition, target: MapPosition) -> FireSolutionRequest<'static> {
    FireSolutionRequest {
        weapon_id: &M252,
        shell_id: &M821_HE,
        gun,
        target,
        wind: Wind::CALM,
    }
}

fn row(solution: &FireSolution, rings: u32) -> ChargeSolution {
    *solution
        .charges
        .iter()
        .find(|charge| charge.rings == rings)
        .expect("the ring has a row")
}

fn refusals(solution: &FireSolution) -> Vec<(u32, Option<SolutionRefusal>)> {
    solution
        .charges
        .iter()
        .map(|charge| (charge.rings, charge.refusal))
        .collect()
}

/// The chord sag of one engine step, `g·Δt²/8` (1.4 mm): in vacuum the step points lie on the
/// exact parabola, so the linear crossing lies at most this far below it.
fn chord_sag_m() -> f64 {
    GRAVITY_M_S2 * DEFAULT_INTEGRATION_STEP_S * DEFAULT_INTEGRATION_STEP_S / 8.0
}

/// The single-precision rounding allowance of a flight lasting `time_s` on a quantity of size
/// `magnitude`: each step's accumulation rounds once to within the `f32` unit roundoff
/// `u = ε/2` of the quantity, plus one rounding of the launch.
fn single_precision_allowance(time_s: f64, magnitude: f64) -> f64 {
    let steps = (time_s / DEFAULT_INTEGRATION_STEP_S).ceil() + 1.0;
    steps * f64::from(f32::EPSILON) / 2.0 * magnitude
}

/// The first-order scheme's bound on a vacuum flight at `elevation_rad` descending through
/// `height_m`: `[range, time of flight, apex]`. The crossing lies at most one chord sag below
/// the parabola, which moves the point of fall by the sag times the horizontal speed over the
/// descent speed of the step and the time by the sag over the descent speed; the highest step
/// point lies within one sag of the apex; each widened by the single-precision allowance.
fn vacuum_scheme_bounds(speed_m_s: f64, elevation_rad: f64, height_m: f64) -> [f64; 3] {
    let (sin, cos) = elevation_rad.sin_cos();
    let vertical = speed_m_s * sin;
    let descent_m_s = (vertical * vertical - 2.0 * GRAVITY_M_S2 * height_m).sqrt()
        - GRAVITY_M_S2 * DEFAULT_INTEGRATION_STEP_S;
    let time_s = (vertical + descent_m_s) / GRAVITY_M_S2;
    let range_m = vacuum_range_m(speed_m_s, elevation_rad, height_m);
    let apex_m = vertical * vertical / (2.0 * GRAVITY_M_S2);
    let height_bound_m = chord_sag_m() + single_precision_allowance(time_s, apex_m);
    [
        chord_sag_m() * speed_m_s * cos / descent_m_s + single_precision_allowance(time_s, range_m),
        height_bound_m / descent_m_s,
        height_bound_m,
    ]
}

/// A bound that still tells a crossing interpolated on its step from one a step late or early:
/// a tenth of the ground one step covers.
fn assert_discriminating(range_bound_m: f64, speed_m_s: f64, elevation_rad: f64) {
    let one_step_m = speed_m_s * elevation_rad.cos() * DEFAULT_INTEGRATION_STEP_S;
    assert!(
        range_bound_m < 0.1 * one_step_m,
        "range bound {range_bound_m} m against one step's {one_step_m} m"
    );
}

/// Horizontal range of a vacuum flight at `elevation_rad` descending through `height_m`.
fn vacuum_range_m(speed_m_s: f64, elevation_rad: f64, height_m: f64) -> f64 {
    let (sin, cos) = elevation_rad.sin_cos();
    let vertical = speed_m_s * sin;
    let time_s =
        (vertical + (vertical * vertical - 2.0 * GRAVITY_M_S2 * height_m).sqrt()) / GRAVITY_M_S2;
    speed_m_s * cos * time_s
}

#[test]
fn vacuum_limit_matches_the_analytic_high_angle_root() {
    let catalog = vacuum_catalog();
    let distance_m = 400.0;
    let solution = solve_fire_solution(
        &catalog,
        &m252_request(at(1000.0, 2000.0, 20.0), at(1000.0, 2400.0, 20.0)),
    )
    .expect("the request is valid");
    assert_eq!(solution.distance_m, distance_m);
    assert_eq!(solution.height_difference_m, 0.0);
    for (rings, speed_m_s) in [(0, 70.0), (1, 70.0 * 1.6), (2, 70.0 * 2.2)] {
        let charge = row(&solution, rings);
        assert_eq!(charge.refusal, None, "ring {rings} solves");
        let solved_rad = degrees_to_radians(charge.elevation_deg.expect("solved"));
        assert!(solved_rad > FRAC_PI_4, "ring {rings}: high-angle branch");
        let [range_bound, time_bound, apex_bound] =
            vacuum_scheme_bounds(speed_m_s, solved_rad, 0.0);
        assert_discriminating(range_bound, speed_m_s, solved_rad);
        let range_m = vacuum_range_m(speed_m_s, solved_rad, 0.0);
        assert!(
            (range_m - distance_m).abs() <= range_bound,
            "ring {rings}: analytic range {range_m} m at the solved elevation, bound {range_bound}"
        );
        let exact_time_s = 2.0 * speed_m_s * solved_rad.sin() / GRAVITY_M_S2;
        let time_s = charge.time_of_flight_s.expect("solved");
        assert!(
            (time_s - exact_time_s).abs() <= time_bound,
            "ring {rings}: time {time_s} vs {exact_time_s} s, bound {time_bound}"
        );
        let exact_apex_m = (speed_m_s * solved_rad.sin()).powi(2) / (2.0 * GRAVITY_M_S2);
        let apex_m = charge.apex_m.expect("solved");
        assert!(
            (apex_m - exact_apex_m).abs() <= apex_bound,
            "ring {rings}: apex {apex_m} vs {exact_apex_m} m, bound {apex_bound}"
        );
    }
}

#[test]
fn vacuum_limit_with_a_height_difference_lands_on_the_analytic_range() {
    let catalog = vacuum_catalog();
    let solution = solve_fire_solution(
        &catalog,
        &m252_request(at(0.0, 0.0, 10.0), at(600.0, 0.0, 60.0)),
    )
    .expect("the request is valid");
    assert_eq!(solution.height_difference_m, 50.0);
    for (rings, speed_m_s) in [(1, 70.0 * 1.6), (2, 70.0 * 2.2)] {
        let charge = row(&solution, rings);
        let solved_rad = degrees_to_radians(charge.elevation_deg.expect("solved"));
        let [range_bound, ..] = vacuum_scheme_bounds(speed_m_s, solved_rad, 50.0);
        assert_discriminating(range_bound, speed_m_s, solved_rad);
        let range_m = vacuum_range_m(speed_m_s, solved_rad, 50.0);
        assert!(
            (range_m - 600.0).abs() <= range_bound,
            "ring {rings}: {range_m} m, bound {range_bound}"
        );
    }
}

#[test]
fn golden_section_finds_the_vacuum_maximum_range_elevation() {
    let mut evaluations = 0;
    let maximum = maximise_by_golden_section(
        |elevation_rad: f64| {
            evaluations += 1;
            (2.0 * elevation_rad).sin()
        },
        0.1,
        1.4,
        GOLDEN_SECTION_ITERATIONS,
    );
    assert!((maximum.argument - FRAC_PI_4).abs() < 1e-7, "{maximum:?}");
    assert_eq!(evaluations, 2 + GOLDEN_SECTION_ITERATIONS);
}

#[test]
fn golden_section_leaves_a_valueless_lower_region_behind() {
    let maximum = maximise_by_golden_section(
        |x: f64| if x < 0.9 { f64::NEG_INFINITY } else { -x },
        0.0,
        1.0,
        GOLDEN_SECTION_ITERATIONS,
    );
    assert!((maximum.argument - 0.9).abs() < 1e-6, "{maximum:?}");
    let nothing = maximise_by_golden_section(|_| f64::NAN, 0.0, 1.0, 5);
    assert_eq!(nothing.value, f64::NEG_INFINITY);
}

#[test]
fn a_target_nearer_than_the_maximum_elevation_range_is_too_close() {
    let solution = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 50.0, 0.0)),
    )
    .expect("the request is valid");
    assert_eq!(
        refusals(&solution),
        vec![
            (0, Some(SolutionRefusal::TooClose)),
            (1, Some(SolutionRefusal::TooClose)),
            (2, Some(SolutionRefusal::TooClose)),
        ]
    );
    assert_eq!(solution.recommended_rings, None);
    assert_eq!(solution.recommended_charge(), None);
    let at_the_gun = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(5.0, 5.0, 0.0), at(5.0, 5.0, 0.0)),
    )
    .expect("the request is valid");
    assert_eq!(at_the_gun.distance_m, 0.0);
    assert!(
        at_the_gun
            .charges
            .iter()
            .all(|charge| charge.refusal == Some(SolutionRefusal::TooClose))
    );
}

#[test]
fn a_target_beyond_the_maximum_range_is_out_of_range() {
    let solution = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 3000.0, 0.0)),
    )
    .expect("the request is valid");
    assert!(solution.charges.iter().all(|charge| charge.refusal
        == Some(SolutionRefusal::OutOfRange)
        && charge.elevation_deg.is_none()
        && charge.elevation_mils.is_none()
        && charge.time_of_flight_s.is_none()
        && charge.apex_m.is_none()));
    assert_eq!(solution.recommended_rings, None);
}

#[test]
fn a_target_above_the_apex_is_unreachable() {
    let solution = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 400.0, 300.0)),
    )
    .expect("the request is valid");
    // Ring 0 peaks at (70 · sin 85.3°)² / 2g ≈ 248 m, below the 300 m target.
    assert_eq!(
        row(&solution, 0).refusal,
        Some(SolutionRefusal::Unreachable)
    );
    assert_eq!(row(&solution, 1).refusal, None);
    assert_eq!(solution.recommended_rings, Some(1));
}

#[test]
fn a_flight_outlasting_the_shell_lifetime_is_refused() {
    let mut catalog = vacuum_catalog();
    catalog.shells[0].time_to_live_s = 5.0;
    let solution = solve_fire_solution(
        &catalog,
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 400.0, 0.0)),
    )
    .expect("the request is valid");
    assert!(
        solution
            .charges
            .iter()
            .all(|charge| charge.refusal == Some(SolutionRefusal::TimeToLiveExceeded))
    );
}

#[test]
fn the_root_search_stops_at_its_iteration_bound_on_a_pathological_function() {
    // A step function over a bracket so wide that sixty halvings leave it far above the
    // tolerance (and whose root sits where one ULP exceeds the tolerance): every interpolation
    // degenerates to bisection and the cap is reached.
    assert_eq!(
        RootSearchLimits::ELEVATION,
        RootSearchLimits {
            max_iterations: 60,
            tolerance: 1e-9,
        }
    );
    let mut evaluations = 0;
    let step = |x: f64| -> Result<f64, ()> {
        evaluations += 1;
        Ok(if x < 123_456_789.0 { 1.0 } else { -1.0 })
    };
    let result = find_bracketed_root(step, (0.0, 1.0), (1e12, -1.0), RootSearchLimits::ELEVATION);
    match result {
        Err(RootSearchError::DidNotConverge {
            iterations,
            bracket_width,
        }) => {
            assert_eq!(iterations, RootSearchLimits::ELEVATION.max_iterations);
            assert!(bracket_width > RootSearchLimits::ELEVATION.tolerance);
        }
        other => panic!("expected DidNotConverge, got {other:?}"),
    }
    assert_eq!(evaluations, RootSearchLimits::ELEVATION.max_iterations);
}

#[test]
fn the_root_search_converges_inside_the_bound_on_a_well_posed_bracket() {
    let mut evaluations = 0;
    let root = find_bracketed_root(
        |x: f64| -> Result<f64, ()> {
            evaluations += 1;
            Ok(x * x * x - 2.0)
        },
        (0.0, -2.0),
        (2.0, 6.0),
        RootSearchLimits::ELEVATION,
    )
    .expect("the bracket holds a root");
    assert!((root - 2f64.cbrt()).abs() < 1e-9, "{root}");
    assert!(evaluations < 20, "{evaluations} evaluations");
    let step_root = find_bracketed_root(
        |x: f64| -> Result<f64, ()> { Ok(if x < 0.25 { 1.0 } else { -1.0 }) },
        (0.0, 1.0),
        (1.0, -1.0),
        RootSearchLimits::ELEVATION,
    )
    .expect("sixty halvings of a unit bracket reach the tolerance");
    assert!((step_root - 0.25).abs() <= 1e-9);
}

/// The single-precision flight's range is a step function of the elevation at the scale of
/// one `f32` rounding of the angle (1.2e-7 rad near 1.2 rad); the search still closes its
/// 1e-9 rad bracket inside its iteration bound on the high-angle bracket of every ring and
/// distance of the drag catalog.
#[test]
fn the_root_search_converges_on_the_single_precision_flight() {
    let catalog = drag_catalog();
    let (low, high) = (FRAC_PI_4, degrees_to_radians(85.3));
    let mut solved = 0;
    for rings in [0, 1, 2] {
        let firing = catalog
            .resolve_firing(&WeaponId::new("m252"), &ShellId::new("m821-he"), rings)
            .expect("the charge exists");
        let range_at = |elevation_rad: f64| {
            let launch = Launch {
                muzzle_speed_m_s: firing.muzzle_speed_m_s,
                elevation_rad,
                azimuth_rad: 0.0,
            };
            fly_to_height(
                &firing.flight_parameters,
                &launch,
                &Wind::CALM,
                0.0,
                PathRecording::Discard,
            )
            .map(|outcome| outcome.downrange_m)
        };
        let (Ok(low_range), Ok(high_range)) = (range_at(low), range_at(high)) else {
            continue;
        };
        for fraction in [0.1, 0.25, 0.4, 0.55, 0.7, 0.85, 0.99] {
            let distance_m = high_range + fraction * (low_range - high_range);
            let mut evaluations = 0;
            let root = find_bracketed_root(
                |elevation_rad: f64| {
                    evaluations += 1;
                    range_at(elevation_rad).map(|range_m| range_m - distance_m)
                },
                (low, low_range - distance_m),
                (high, high_range - distance_m),
                RootSearchLimits::ELEVATION,
            );
            assert!(
                root.is_ok() && evaluations <= RootSearchLimits::ELEVATION.max_iterations,
                "ring {rings} at {distance_m} m: {root:?} after {evaluations} evaluations"
            );
            solved += 1;
        }
    }
    assert_eq!(solved, 21, "every ring's bracket was flown");
}

#[test]
fn the_root_search_refuses_brackets_without_a_sign_change_and_non_finite_values() {
    let same_sign = find_bracketed_root(
        |_| -> Result<f64, ()> { Ok(1.0) },
        (0.0, 1.0),
        (1.0, 2.0),
        RootSearchLimits::ELEVATION,
    );
    assert!(matches!(
        same_sign,
        Err(RootSearchError::NoSignChange { .. })
    ));
    let nan_end = find_bracketed_root(
        |_| -> Result<f64, ()> { Ok(1.0) },
        (0.0, f64::NAN),
        (1.0, -1.0),
        RootSearchLimits::ELEVATION,
    );
    assert!(matches!(nan_end, Err(RootSearchError::NoSignChange { .. })));
    let nan_inside = find_bracketed_root(
        |_| -> Result<f64, ()> { Ok(f64::NAN) },
        (0.0, 1.0),
        (1.0, -1.0),
        RootSearchLimits::ELEVATION,
    );
    assert!(matches!(
        nan_inside,
        Err(RootSearchError::NonFiniteValue { .. })
    ));
    let refused = find_bracketed_root(
        |_| -> Result<f64, &'static str> { Err("no flight") },
        (0.0, 1.0),
        (1.0, -1.0),
        RootSearchLimits::ELEVATION,
    );
    assert_eq!(refused, Err(RootSearchError::Evaluation("no flight")));
}

#[test]
fn the_recommended_ring_is_the_lowest_ring_that_solves() {
    let solution = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 1000.0, 0.0)),
    )
    .expect("the request is valid");
    assert_eq!(
        refusals(&solution),
        vec![(0, Some(SolutionRefusal::OutOfRange)), (1, None), (2, None)]
    );
    assert_eq!(solution.recommended_rings, Some(1));
    assert_eq!(
        solution.recommended_charge().map(|charge| charge.rings),
        Some(1)
    );

    let solved = |rings| ChargeSolution {
        rings,
        elevation_deg: Some(60.0),
        elevation_mils: Some(1066.7),
        time_of_flight_s: Some(20.0),
        apex_m: Some(300.0),
        aim_azimuth_deg: Some(90.0),
        aim_azimuth_mils: Some(1600.0),
        deflection_correction_mils: Some(0.0),
        range_correction_m: Some(0.0),
        refusal: None,
    };
    let refused = ChargeSolution {
        refusal: Some(SolutionRefusal::TooClose),
        ..solved(0)
    };
    assert_eq!(recommended_rings(&[solved(4), solved(2), refused]), Some(2));
    assert_eq!(recommended_rings(&[refused]), None);
    assert_eq!(recommended_rings(&[]), None);
}

#[test]
fn the_solved_elevation_flies_to_the_target_with_drag_and_wind() {
    let catalog = drag_catalog();
    let wind = Wind {
        speed_m_s: 6.0,
        from_deg: 250.0,
    };
    let request = FireSolutionRequest {
        wind,
        ..m252_request(at(100.0, 100.0, 40.0), at(700.0, 500.0, 15.0))
    };
    let solution = solve_fire_solution(&catalog, &request).expect("the request is valid");
    let resolved = catalog
        .resolve_firing(&WeaponId::new("m252"), &ShellId::new("m821-he"), 2)
        .expect("ring 2 exists");
    let charge = row(&solution, 2);
    assert_eq!(charge.refusal, None);
    let launch = Launch {
        muzzle_speed_m_s: resolved.muzzle_speed_m_s,
        elevation_rad: degrees_to_radians(charge.elevation_deg.expect("solved")),
        azimuth_rad: degrees_to_radians(charge.aim_azimuth_deg.expect("solved")),
    };
    let flight = fly_to_height(
        &resolved.flight_parameters,
        &launch,
        &wind,
        -25.0,
        PathRecording::Discard,
    )
    .expect("the solved aim flies");
    let miss_m = libm::hypot(
        100.0 + flight.impact_position_m[0] - 700.0,
        100.0 + flight.impact_position_m[1] - 500.0,
    );
    assert!(miss_m < 0.05, "the solved aim misses by {miss_m} m");
    assert!((flight.time_of_flight_s - charge.time_of_flight_s.expect("solved")).abs() < 1e-6);
}

#[test]
fn malformed_requests_are_refused_without_panicking() {
    let catalog = vacuum_catalog();
    let good = m252_request(at(0.0, 0.0, 0.0), at(0.0, 400.0, 0.0));
    let invalid = |parameter: &'static str| {
        move |result: Result<FireSolution, FireSolutionError>| match result {
            Err(FireSolutionError::InvalidInput {
                parameter: refused, ..
            }) => refused == parameter,
            _ => false,
        }
    };
    let with_gun = |gun| FireSolutionRequest { gun, ..good };
    let with_target = |target| FireSolutionRequest { target, ..good };
    assert!(invalid("gun.x_m")(solve_fire_solution(
        &catalog,
        &with_gun(at(f64::NAN, 0.0, 0.0))
    )));
    assert!(invalid("gun.height_m")(solve_fire_solution(
        &catalog,
        &with_gun(at(0.0, 0.0, f64::NEG_INFINITY))
    )));
    assert!(invalid("target.y_m")(solve_fire_solution(
        &catalog,
        &with_target(at(0.0, f64::INFINITY, 0.0))
    )));
    assert!(invalid("distance_m")(solve_fire_solution(
        &catalog,
        &with_target(at(f64::MAX, f64::MAX, 0.0))
    )));
    let bad_wind = FireSolutionRequest {
        wind: Wind {
            speed_m_s: f64::NAN,
            from_deg: 0.0,
        },
        ..good
    };
    assert!(matches!(
        solve_fire_solution(&catalog, &bad_wind),
        Err(FireSolutionError::InvalidWind(WindError::InvalidSpeed(_)))
    ));
    let unknown = FireSolutionRequest {
        weapon_id: &WeaponId::new("M252"),
        ..good
    };
    assert!(matches!(
        solve_fire_solution(&catalog, &unknown),
        Err(FireSolutionError::Lookup(
            CatalogLookupError::UnknownWeapon { .. }
        ))
    ));
}

#[test]
fn degenerate_catalog_values_are_refused_without_panicking() {
    let good = m252_request(at(0.0, 0.0, 0.0), at(0.0, 400.0, 0.0));
    let solve_with = |edit: &dyn Fn(&mut BallisticsCatalog)| {
        let mut catalog = vacuum_catalog();
        edit(&mut catalog);
        solve_fire_solution(&catalog, &good)
    };
    assert!(matches!(
        solve_with(&|catalog| catalog.shells[0].mass_kg = -4.2),
        Err(FireSolutionError::InvalidFlightParameters(_))
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.gravity_m_s2 = 0.0),
        Err(FireSolutionError::InvalidFlightParameters(_))
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.shells[0].air_drag = f64::NAN),
        Err(FireSolutionError::InvalidFlightParameters(_))
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.weapons[0].mils_per_circle = 0),
        Err(FireSolutionError::InvalidMilsConvention(_))
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.weapons[0].elevation_min_deg = 86.0),
        Err(FireSolutionError::InvalidInput {
            parameter: "elevation_max_deg",
            ..
        })
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.weapons[0].elevation_max_deg = f64::NAN),
        Err(FireSolutionError::InvalidInput {
            parameter: "elevation_max_deg",
            ..
        })
    ));
    assert!(matches!(
        solve_with(&|catalog| catalog.weapons[0].elevation_max_deg = 91.0),
        Err(FireSolutionError::InvalidInput {
            parameter: "elevation_max_deg",
            ..
        })
    ));
    let zero_speed = solve_with(&|catalog| catalog.shells[0].init_speed_m_s = 0.0)
        .expect("a zero muzzle speed is a per-charge refusal");
    assert!(
        zero_speed
            .charges
            .iter()
            .all(|charge| charge.refusal == Some(SolutionRefusal::InvalidInput))
    );
    let nan_coefficient =
        solve_with(&|catalog| catalog.shells[0].charges[1].init_speed_coef = f64::NAN)
            .expect("a NaN charge coefficient is that charge's refusal");
    assert_eq!(
        row(&nan_coefficient, 1).refusal,
        Some(SolutionRefusal::InvalidInput)
    );
    assert_eq!(row(&nan_coefficient, 0).refusal, None);
}

#[test]
fn a_charge_problem_with_non_finite_geometry_is_invalid_input() {
    let catalog = vacuum_catalog();
    let resolved = catalog
        .resolve_firing(&WeaponId::new("m252"), &ShellId::new("m821-he"), 1)
        .expect("ring 1 exists");
    let good = ChargeProblem {
        flight_parameters: resolved.flight_parameters,
        muzzle_speed_m_s: resolved.muzzle_speed_m_s,
        elevation_limits_rad: resolved.weapon.elevation_limits_rad(),
        azimuth_rad: 0.0,
        distance_m: 800.0,
        height_difference_m: 0.0,
        wind: Wind::CALM,
    };
    assert!(solve_charge_elevation(&good).is_ok());
    for problem in [
        ChargeProblem {
            distance_m: f64::NAN,
            ..good
        },
        ChargeProblem {
            distance_m: -1.0,
            ..good
        },
        ChargeProblem {
            azimuth_rad: f64::INFINITY,
            ..good
        },
        ChargeProblem {
            elevation_limits_rad: (1.2, 0.9),
            ..good
        },
        ChargeProblem {
            elevation_limits_rad: (f64::NAN, 1.4),
            ..good
        },
        ChargeProblem {
            muzzle_speed_m_s: f64::NAN,
            ..good
        },
        ChargeProblem {
            height_difference_m: f64::INFINITY,
            ..good
        },
    ] {
        assert_eq!(
            solve_charge_elevation(&problem),
            Err(SolutionRefusal::InvalidInput),
            "{problem:?}"
        );
    }
}

#[test]
fn azimuth_cardinals_follow_each_weapon_mils_convention() {
    let catalog = vacuum_catalog();
    let cardinals = [
        ((0.0, 400.0), 0.0),
        ((400.0, 0.0), 90.0),
        ((0.0, -400.0), 180.0),
        ((-400.0, 0.0), 270.0),
    ];
    for (weapon_id, shell_id, mils_per_circle) in
        [("m252", "m821-he", 6400.0), ("2b14", "o-832-he", 6000.0)]
    {
        let (weapon_id, shell_id) = (&WeaponId::new(weapon_id), &ShellId::new(shell_id));
        for ((east_m, north_m), azimuth_deg) in cardinals {
            let request = FireSolutionRequest {
                weapon_id,
                shell_id,
                ..m252_request(
                    at(50.0, -20.0, 0.0),
                    at(50.0 + east_m, -20.0 + north_m, 0.0),
                )
            };
            let solution = solve_fire_solution(&catalog, &request).expect("the request is valid");
            assert_eq!(solution.mils_per_circle as f64, mils_per_circle);
            assert!(
                (solution.azimuth_deg - azimuth_deg).abs() < 1e-9,
                "{weapon_id} {azimuth_deg}: {}",
                solution.azimuth_deg
            );
            let expected_mils = azimuth_deg * mils_per_circle / 360.0;
            assert!(
                (solution.azimuth_mils - expected_mils).abs() < 1e-9,
                "{weapon_id} {azimuth_deg}: {}",
                solution.azimuth_mils
            );
            let charge = solution.recommended_charge().expect("400 m solves");
            let elevation_deg = charge.elevation_deg.expect("solved");
            let elevation_mils = charge.elevation_mils.expect("solved");
            assert!(
                (elevation_mils - elevation_deg * mils_per_circle / 360.0).abs() < 1e-9,
                "{weapon_id}: {elevation_mils} mils vs {elevation_deg}°"
            );
        }
    }
    let north_west = solve_fire_solution(
        &catalog,
        &m252_request(at(0.0, 0.0, 0.0), at(-300.0, 300.0, 0.0)),
    )
    .expect("the request is valid");
    assert!((north_west.azimuth_deg - 315.0).abs() < 1e-9);
    assert!((north_west.azimuth_mils - 5600.0).abs() < 1e-9);
}

#[test]
fn the_solution_serialises_to_the_contract_field_names() {
    let solution = solve_fire_solution(
        &vacuum_catalog(),
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 3000.0, 0.0)),
    )
    .expect("the request is valid");
    let json = serde_json::to_value(&solution).expect("the solution serialises");
    assert_eq!(json["recommended_rings"], serde_json::Value::Null);
    assert_eq!(json["mils_per_circle"], 6400);
    assert_eq!(json["charges"][0]["refusal"], "out_of_range");
    assert_eq!(
        json["charges"][0]["elevation_mils"],
        serde_json::Value::Null
    );
    let decoded: FireSolution = serde_json::from_value(json).expect("the solution decodes");
    assert_eq!(decoded, solution);
    for (refusal, wire) in [
        (SolutionRefusal::TooClose, "too_close"),
        (SolutionRefusal::OutOfRange, "out_of_range"),
        (SolutionRefusal::Unreachable, "unreachable"),
        (SolutionRefusal::DidNotConverge, "did_not_converge"),
        (SolutionRefusal::TimeToLiveExceeded, "time_to_live_exceeded"),
        (SolutionRefusal::InvalidInput, "invalid_input"),
    ] {
        assert_eq!(serde_json::to_value(refusal).expect("serialises"), wire);
    }
}

#[test]
fn a_maximum_elevation_outlasting_the_lifetime_searches_the_lower_bracket() {
    let mut catalog = vacuum_catalog();
    // Ring 0 (70 m/s) flies 14.2 s at 85.3° but lands 300 m out at 71.6° after 13.5 s.
    catalog.shells[0].time_to_live_s = 14.0;
    let solution = solve_fire_solution(
        &catalog,
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 300.0, 0.0)),
    )
    .expect("the request is valid");
    let charge = row(&solution, 0);
    assert_eq!(charge.refusal, None);
    let solved_rad = degrees_to_radians(charge.elevation_deg.expect("solved"));
    assert!(solved_rad > FRAC_PI_4, "high-angle branch");
    let [range_bound, ..] = vacuum_scheme_bounds(70.0, solved_rad, 0.0);
    assert_discriminating(range_bound, 70.0, solved_rad);
    let range_m = vacuum_range_m(70.0, solved_rad, 0.0);
    assert!(
        (range_m - 300.0).abs() <= range_bound,
        "analytic range {range_m} m at {solved_rad} rad, bound {range_bound}"
    );
    assert!(charge.time_of_flight_s.expect("solved") <= 14.0);
    // Ring 1 lands late even at the lowest elevation.
    assert_eq!(
        row(&solution, 1).refusal,
        Some(SolutionRefusal::TimeToLiveExceeded)
    );

    // 150 m is nearer than the 190 m the highest in-time elevation reaches: every nearer landing
    // comes too late, so the refusal is the lifetime, not the distance.
    let near = solve_fire_solution(
        &catalog,
        &m252_request(at(0.0, 0.0, 0.0), at(0.0, 150.0, 0.0)),
    )
    .expect("the request is valid");
    assert_eq!(
        row(&near, 0).refusal,
        Some(SolutionRefusal::TimeToLiveExceeded)
    );
}

/// The wind-corrected aim cases, sharing this file's catalogs and request helpers.
#[path = "solver_wind_aim.rs"]
mod wind_aim;
