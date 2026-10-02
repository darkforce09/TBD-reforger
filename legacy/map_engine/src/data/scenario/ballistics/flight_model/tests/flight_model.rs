//! Tests of the shell flight model: vacuum analytics within the engine step's bound, drag and
//! step convergence, energy, the linear crossing, refusals, wind response and engine-oracle
//! reference cases.
//!
//! The scheme's own properties (vacuum parabola, chord sag, energy, first order, drag
//! convergence, linear crossing) are proven on the double-precision flight of the same step
//! ([`fly_reference`]), free of single-precision rounding; the production `f32` flight is
//! proven against it and against the engine oracle.

use super::trajectory::fly_to_height_in;
use super::*;
use crate::data::scenario::ballistics::wind::Wind;

/// The gravity the engine oracle of game build 1.8.0.13 reports, as the catalog carries it.
const GRAVITY_M_S2: f64 = 9.81;

/// M821 flight constants from the game export: mass 4.06 kg, air drag 0.000462, lifetime 60 s.
fn m821_parameters() -> FlightParameters {
    FlightParameters {
        gravity_m_s2: GRAVITY_M_S2,
        mass_kg: 4.06,
        air_drag: 0.000462,
        wind_influence_multiplier: 1.0,
        time_to_live_s: 60.0,
        integration_step_s: DEFAULT_INTEGRATION_STEP_S,
    }
}

fn vacuum_parameters() -> FlightParameters {
    FlightParameters {
        air_drag: 0.0,
        ..m821_parameters()
    }
}

fn launch(muzzle_speed_m_s: f64, elevation_deg: f64) -> Launch {
    Launch {
        muzzle_speed_m_s,
        elevation_rad: elevation_deg.to_radians(),
        azimuth_rad: 0.0,
    }
}

fn fly(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    height_m: f64,
) -> FlightOutcome {
    match fly_to_height(parameters, launch, wind, height_m, PathRecording::Discard) {
        Ok(outcome) => outcome,
        Err(error) => panic!("expected a flight for {launch:?} to {height_m} m, got {error}"),
    }
}

/// The same scheme in double precision: the reference the scheme's properties are proven on.
fn fly_reference(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    height_m: f64,
    recording: PathRecording,
) -> Result<FlightOutcome, FlightError> {
    fly_to_height_in::<f64>(parameters, launch, wind, height_m, recording)
}

/// [`fly_reference`] without the path, panicking on a refusal.
fn fly_scheme(
    parameters: &FlightParameters,
    launch: &Launch,
    wind: &Wind,
    height_m: f64,
) -> FlightOutcome {
    match fly_reference(parameters, launch, wind, height_m, PathRecording::Discard) {
        Ok(outcome) => outcome,
        Err(error) => panic!("expected a flight for {launch:?} to {height_m} m, got {error}"),
    }
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    ((actual - expected) / expected).abs()
}

/// The largest height of a vacuum parabola above the chord of one step, `g·Δt²/8`.
///
/// In vacuum the engine step's points lie on the exact parabola, so the linear crossing and the
/// highest step point differ from the analytic flight only by this chord sag.
fn chord_sag_m(step_s: f64) -> f64 {
    GRAVITY_M_S2 * step_s * step_s / 8.0
}

/// Bounds of the vacuum flight's range, time of flight, apex and apex time for a launch at
/// `speed` and `elevation`: the chord crosses the target height at most one sag, divided by the
/// chord's slope, before the parabola; the highest step point is within one sag of the apex
/// and within half a step of its time.
fn vacuum_bounds(speed: f64, elevation: f64, step_s: f64) -> [f64; 4] {
    let sag = chord_sag_m(step_s);
    let descent_speed = speed * elevation.sin() - GRAVITY_M_S2 * step_s;
    let horizontal_speed = speed * elevation.cos();
    [
        sag * horizontal_speed / descent_speed,
        sag / descent_speed,
        sag,
        0.5 * step_s,
    ]
}

#[test]
fn vacuum_flight_matches_analytic_range_time_and_apex_within_the_chord_sag_at_five_angles() {
    let speed = 100.0;
    for elevation_deg in [15.0_f64, 30.0, 45.0, 60.0, 80.0] {
        let elevation = elevation_deg.to_radians();
        let expected_range = speed * speed * (2.0 * elevation).sin() / GRAVITY_M_S2;
        let expected_time = 2.0 * speed * elevation.sin() / GRAVITY_M_S2;
        let vertical = speed * elevation.sin();
        let expected_apex = vertical * vertical / (2.0 * GRAVITY_M_S2);
        let outcome = fly_scheme(
            &vacuum_parameters(),
            &launch(speed, elevation_deg),
            &Wind::CALM,
            0.0,
        );
        let bounds = vacuum_bounds(speed, elevation, DEFAULT_INTEGRATION_STEP_S);
        let cases = [
            ("range", outcome.downrange_m, expected_range, bounds[0]),
            (
                "time of flight",
                outcome.time_of_flight_s,
                expected_time,
                bounds[1],
            ),
            ("apex", outcome.apex_height_m, expected_apex, bounds[2]),
            (
                "apex time",
                outcome.apex_time_s,
                expected_time / 2.0,
                bounds[3],
            ),
        ];
        for (quantity, actual, expected, bound) in cases {
            let error = (actual - expected).abs();
            assert!(
                error <= bound + 1e-9 * expected,
                "{elevation_deg}°: {quantity} {actual} vs analytic {expected} ({error:e} > {bound:e})"
            );
        }
        assert!(
            outcome.downrange_m <= expected_range && outcome.apex_height_m <= expected_apex,
            "{elevation_deg}°: the chord and the step points lie on or under the parabola"
        );
        assert!(
            outcome.deflection_m.abs() <= 1e-9,
            "vacuum flight drifted sideways"
        );
    }
}

#[test]
fn vacuum_step_points_lie_on_the_analytic_parabola() {
    let (speed, elevation_deg) = (120.0, 55.0);
    let outcome = fly_reference(
        &vacuum_parameters(),
        &launch(speed, elevation_deg),
        &Wind::CALM,
        0.0,
        PathRecording::KeepSamples,
    )
    .expect("the vacuum flight lands");
    let elevation = f64::to_radians(elevation_deg);
    let step_points = &outcome.path[..outcome.path.len() - 1];
    assert!(step_points.len() > 100, "the path was not recorded");
    for sample in step_points {
        let t = sample.time_s;
        let north = speed * elevation.cos() * t;
        let up = speed * elevation.sin() * t - 0.5 * GRAVITY_M_S2 * t * t;
        assert!(
            (sample.position_m[1] - north).abs() <= 1e-9 * north.max(1.0)
                && (sample.position_m[2] - up).abs() <= 1e-9 * north.max(1.0),
            "step point at t = {t} s: {:?} vs ({north}, {up})",
            sample.position_m
        );
    }
}

#[test]
fn vacuum_flight_follows_the_launch_azimuth() {
    let azimuth_rad = 2.2;
    let fired = Launch {
        azimuth_rad,
        ..launch(80.0, 50.0)
    };
    let outcome = fly_scheme(&vacuum_parameters(), &fired, &Wind::CALM, 0.0);
    let expected_range = 80.0 * 80.0 * 100.0_f64.to_radians().sin() / GRAVITY_M_S2;
    let bound = vacuum_bounds(80.0, 50.0_f64.to_radians(), DEFAULT_INTEGRATION_STEP_S)[0];
    let [east, north, up] = outcome.impact_position_m;
    assert!((outcome.downrange_m - expected_range).abs() <= bound);
    assert!(outcome.deflection_m.abs() <= 1e-9);
    assert!((east - expected_range * azimuth_rad.sin()).abs() <= bound);
    assert!((north - expected_range * azimuth_rad.cos()).abs() <= bound);
    assert_eq!(up, 0.0);
}

#[test]
fn small_drag_converges_to_the_vacuum_flight() {
    let fired = launch(150.0, 60.0);
    let vacuum = fly_scheme(&vacuum_parameters(), &fired, &Wind::CALM, 0.0);
    let mut previous_error = f64::INFINITY;
    for air_drag in [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9] {
        let parameters = FlightParameters {
            air_drag,
            ..m821_parameters()
        };
        let outcome = fly_scheme(&parameters, &fired, &Wind::CALM, 0.0);
        let error = relative_error(outcome.downrange_m, vacuum.downrange_m)
            .max(relative_error(
                outcome.time_of_flight_s,
                vacuum.time_of_flight_s,
            ))
            .max(relative_error(outcome.apex_height_m, vacuum.apex_height_m));
        assert!(
            outcome.downrange_m < vacuum.downrange_m,
            "drag {air_drag} did not shorten the range"
        );
        assert!(
            error < previous_error,
            "drag {air_drag}: error {error:e} did not shrink from {previous_error:e}"
        );
        previous_error = error;
    }
    assert!(
        previous_error <= 1e-6,
        "smallest drag still differs by {previous_error:e}"
    );
}

/// Every shell of the vanilla catalog at every charge coefficient, elevation 45° to 85°.
fn case_lattice() -> Vec<(&'static str, FlightParameters, f64)> {
    let shells: [(&str, f64, f64, f64, &[f64]); 7] = [
        (
            "M821",
            66.0,
            4.06,
            0.000462,
            &[1.0, 1.531, 2.085, 2.541, 2.977],
        ),
        (
            "M879",
            66.0,
            4.26,
            0.000469,
            &[1.0, 1.573, 2.082, 2.532, 2.932],
        ),
        (
            "M819",
            137.0,
            4.85,
            0.0009139,
            &[0.666, 0.959, 1.184, 1.387],
        ),
        ("M853A1", 152.0, 4.0, 0.001488, &[0.638, 1.0, 1.281, 1.596]),
        (
            "O-832DU",
            76.0,
            3.10,
            0.000615,
            &[1.0, 1.321, 1.736, 2.087, 2.455],
        ),
        ("D-832DU", 71.0, 3.48, 0.000655, &[1.0, 1.339, 1.748, 2.086]),
        (
            "S-832S",
            127.0,
            3.51,
            0.001836,
            &[0.698, 1.111, 1.512, 2.154],
        ),
    ];
    let mut cases = Vec::new();
    for (name, init_speed, mass_kg, air_drag, coefficients) in shells {
        let parameters = FlightParameters {
            mass_kg,
            air_drag,
            ..m821_parameters()
        };
        for coefficient in coefficients {
            cases.push((name, parameters, init_speed * coefficient));
        }
    }
    cases
}

#[test]
fn the_engine_step_is_first_order_across_the_case_lattice() {
    let refined = |parameters: &FlightParameters, divisor: f64| FlightParameters {
        integration_step_s: parameters.integration_step_s / divisor,
        ..*parameters
    };
    for (name, parameters, speed) in case_lattice() {
        let (half, quarter) = (refined(&parameters, 2.0), refined(&parameters, 4.0));
        for elevation_deg in [45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 85.0] {
            let fired = launch(speed, elevation_deg);
            let flights = [&parameters, &half, &quarter]
                .map(|flown| fly_scheme(flown, &fired, &Wind::CALM, 0.0));
            let label = format!("{name} at {speed} m/s, {elevation_deg}°");
            for (quantity, values) in [
                ("range", flights.each_ref().map(|flight| flight.downrange_m)),
                (
                    "time",
                    flights.each_ref().map(|flight| flight.time_of_flight_s),
                ),
            ] {
                let ratio = (values[0] - values[1]) / (values[1] - values[2]);
                assert!(
                    (1.8..=2.2).contains(&ratio),
                    "{label}: {quantity} error ratio {ratio} under step halving is not first order"
                );
            }
        }
    }
}

#[test]
fn mechanical_energy_decreases_at_every_step_under_drag() {
    let parameters = m821_parameters();
    let outcome = fly_to_height(
        &parameters,
        &launch(66.0 * 2.977, 70.0),
        &Wind::CALM,
        0.0,
        PathRecording::KeepSamples,
    )
    .expect("the reference flight lands");
    assert!(outcome.path.len() > 100, "the path was not recorded");
    let energy = |sample: &FlightSample| {
        let [vx, vy, vz] = sample.velocity_m_s;
        0.5 * (vx * vx + vy * vy + vz * vz) + GRAVITY_M_S2 * sample.position_m[2]
    };
    for pair in outcome.path.windows(2) {
        assert!(
            energy(&pair[1]) < energy(&pair[0]),
            "energy rose between t = {} s and t = {} s",
            pair[0].time_s,
            pair[1].time_s
        );
    }
}

#[test]
fn vacuum_flight_conserves_mechanical_energy() {
    let outcome = fly_reference(
        &vacuum_parameters(),
        &launch(120.0, 55.0),
        &Wind::CALM,
        0.0,
        PathRecording::KeepSamples,
    )
    .expect("the vacuum flight lands");
    let initial = 0.5 * 120.0 * 120.0;
    let energy = |sample: &FlightSample| {
        let [vx, vy, vz] = sample.velocity_m_s;
        0.5 * (vx * vx + vy * vy + vz * vz) + GRAVITY_M_S2 * sample.position_m[2]
    };
    let (crossing, step_points) = outcome.path.split_last().expect("recorded path");
    for sample in step_points {
        assert!(
            relative_error(energy(sample), initial) <= 1e-9,
            "energy {} at t = {}",
            energy(sample),
            sample.time_s
        );
    }
    let crossing_bound = GRAVITY_M_S2 * chord_sag_m(DEFAULT_INTEGRATION_STEP_S) / initial;
    assert!(
        relative_error(energy(crossing), initial) <= crossing_bound,
        "energy {} at the crossing",
        energy(crossing)
    );
}

#[test]
fn recorded_path_is_time_ordered_and_ends_at_the_crossing() {
    let outcome = fly_to_height(
        &m821_parameters(),
        &launch(66.0, 45.0),
        &Wind::CALM,
        -12.0,
        PathRecording::KeepSamples,
    )
    .expect("the flight reaches the lower target");
    let first = outcome.path.first().expect("muzzle sample");
    let last = outcome.path.last().expect("crossing sample");
    assert_eq!(first.time_s, 0.0);
    assert_eq!(first.position_m, [0.0; 3]);
    assert_eq!(last.time_s, outcome.time_of_flight_s);
    assert_eq!(last.position_m, outcome.impact_position_m);
    assert!(
        outcome
            .path
            .windows(2)
            .all(|pair| pair[0].time_s < pair[1].time_s)
    );
    let discarded = fly(&m821_parameters(), &launch(66.0, 45.0), &Wind::CALM, -12.0);
    assert!(discarded.path.is_empty());
    assert_eq!(discarded.time_of_flight_s, outcome.time_of_flight_s);
}

#[test]
fn the_crossing_interpolates_its_step_linearly_and_the_apex_is_the_highest_step_point() {
    let parameters = m821_parameters();
    for (elevation_deg, speed, height_m) in
        [(45.0, 66.0, 0.0), (60.0, 150.0, 40.0), (80.0, 120.0, -25.0)]
    {
        let fired = launch(speed, elevation_deg);
        let record = |target_m: f64| {
            fly_reference(
                &parameters,
                &fired,
                &Wind::CALM,
                target_m,
                PathRecording::KeepSamples,
            )
            .expect("the flight lands")
        };
        let outcome = record(height_m);
        let deeper = record(height_m - 1000.0);
        let steps_before = outcome.path.len() - 1;
        let (start, end) = (&deeper.path[steps_before - 1], &deeper.path[steps_before]);
        assert_eq!(start, &outcome.path[steps_before - 1], "same step points");
        let s = (start.position_m[2] - height_m) / (start.position_m[2] - end.position_m[2]);
        let label = format!("{elevation_deg}° at {speed} m/s to {height_m} m");
        assert!((0.0..=1.0).contains(&s), "{label}: fraction {s}");
        let expected_time = start.time_s + s * (end.time_s - start.time_s);
        assert!(
            (outcome.time_of_flight_s - expected_time).abs() <= 1e-9,
            "{label}: {} vs {expected_time} s",
            outcome.time_of_flight_s
        );
        for axis in 0..2 {
            let expected =
                start.position_m[axis] + s * (end.position_m[axis] - start.position_m[axis]);
            assert!(
                (outcome.impact_position_m[axis] - expected).abs()
                    <= 1e-9 * expected.abs().max(1.0),
                "{label}: {:?} off the chord at axis {axis}",
                outcome.impact_position_m
            );
        }
        let highest = deeper
            .path
            .iter()
            .max_by(|a, b| a.position_m[2].total_cmp(&b.position_m[2]))
            .expect("recorded path");
        assert_eq!(
            outcome.apex_height_m, highest.position_m[2],
            "{label}: apex"
        );
        assert_eq!(outcome.apex_time_s, highest.time_s, "{label}: apex time");
    }
}

#[test]
fn a_flight_longer_than_the_time_to_live_is_refused() {
    let fired = launch(66.0, 45.0);
    for time_to_live_s in [5.0, 9.4] {
        let parameters = FlightParameters {
            time_to_live_s,
            ..m821_parameters()
        };
        assert_eq!(
            fly_to_height(
                &parameters,
                &fired,
                &Wind::CALM,
                0.0,
                PathRecording::Discard
            ),
            Err(FlightError::TimeToLiveExceeded { time_to_live_s })
        );
    }
    let parameters = FlightParameters {
        time_to_live_s: 9.45,
        ..m821_parameters()
    };
    assert!(
        fly_to_height(
            &parameters,
            &fired,
            &Wind::CALM,
            0.0,
            PathRecording::Discard
        )
        .is_ok()
    );
}

#[test]
fn a_target_above_the_apex_is_refused() {
    let result = fly_to_height(
        &m821_parameters(),
        &launch(66.0, 45.0),
        &Wind::CALM,
        120.0,
        PathRecording::Discard,
    );
    match result {
        Err(FlightError::TargetAboveApex {
            apex_height_m,
            target_height_m,
        }) => {
            assert_eq!(target_height_m, 120.0);
            assert!(
                (apex_height_m - 108.73).abs() <= 0.5,
                "apex {apex_height_m}"
            );
        }
        other => panic!("expected TargetAboveApex, got {other:?}"),
    }
}

#[test]
fn invalid_inputs_are_refused_without_panicking() {
    let fired = launch(66.0, 45.0);
    let base = m821_parameters();
    let refused_parameters = [
        (
            "mass_kg",
            FlightParameters {
                mass_kg: -4.0,
                ..base
            },
        ),
        (
            "mass_kg",
            FlightParameters {
                mass_kg: f64::NAN,
                ..base
            },
        ),
        (
            "gravity_m_s2",
            FlightParameters {
                gravity_m_s2: f64::INFINITY,
                ..base
            },
        ),
        (
            "air_drag",
            FlightParameters {
                air_drag: -1e-4,
                ..base
            },
        ),
        (
            "wind_influence_multiplier",
            FlightParameters {
                wind_influence_multiplier: f64::NAN,
                ..base
            },
        ),
        (
            "time_to_live_s",
            FlightParameters {
                time_to_live_s: 0.0,
                ..base
            },
        ),
        (
            "integration_step_s",
            FlightParameters {
                integration_step_s: 0.0,
                ..base
            },
        ),
    ];
    for (parameter, parameters) in refused_parameters {
        let result = fly_to_height(
            &parameters,
            &fired,
            &Wind::CALM,
            0.0,
            PathRecording::Discard,
        );
        assert!(
            matches!(result, Err(FlightError::InvalidInput { parameter: named, .. }) if named == parameter),
            "{parameter}: {result:?}"
        );
    }
    let refused_launches = [
        (
            "muzzle_speed_m_s",
            Launch {
                muzzle_speed_m_s: 0.0,
                ..fired
            },
        ),
        (
            "elevation_rad",
            Launch {
                elevation_rad: f64::NAN,
                ..fired
            },
        ),
        (
            "azimuth_rad",
            Launch {
                azimuth_rad: f64::NEG_INFINITY,
                ..fired
            },
        ),
    ];
    for (parameter, bad_launch) in refused_launches {
        let result = fly_to_height(&base, &bad_launch, &Wind::CALM, 0.0, PathRecording::Discard);
        assert!(
            matches!(result, Err(FlightError::InvalidInput { parameter: named, .. }) if named == parameter),
            "{parameter}: {result:?}"
        );
    }
    let result = fly_to_height(&base, &fired, &Wind::CALM, f64::NAN, PathRecording::Discard);
    assert!(matches!(
        result,
        Err(FlightError::InvalidInput {
            parameter: "target_height_m",
            ..
        })
    ));
    let bad_wind = Wind {
        speed_m_s: -1.0,
        from_deg: 0.0,
    };
    let result = fly_to_height(&base, &fired, &bad_wind, 0.0, PathRecording::Discard);
    assert!(
        matches!(result, Err(FlightError::InvalidWind(_))),
        "{result:?}"
    );
    let endless = FlightParameters {
        time_to_live_s: 1e9,
        ..base
    };
    let result = fly_to_height(&endless, &fired, &Wind::CALM, 0.0, PathRecording::Discard);
    assert!(
        matches!(result, Err(FlightError::TooManyIntegrationSteps { .. })),
        "{result:?}"
    );
}

#[test]
fn a_headwind_shortens_and_a_tailwind_lengthens_the_range() {
    let fired = launch(66.0 * 2.085, 60.0);
    let calm = fly(&m821_parameters(), &fired, &Wind::CALM, 0.0);
    let headwind = fly(
        &m821_parameters(),
        &fired,
        &Wind {
            speed_m_s: 8.0,
            from_deg: 0.0,
        },
        0.0,
    );
    let tailwind = fly(
        &m821_parameters(),
        &fired,
        &Wind {
            speed_m_s: 8.0,
            from_deg: 180.0,
        },
        0.0,
    );
    assert!(
        headwind.downrange_m < calm.downrange_m - 1.0,
        "headwind {} vs calm {}",
        headwind.downrange_m,
        calm.downrange_m
    );
    assert!(
        tailwind.downrange_m > calm.downrange_m + 1.0,
        "tailwind {} vs calm {}",
        tailwind.downrange_m,
        calm.downrange_m
    );
    assert!(headwind.deflection_m.abs() <= 1e-9 && tailwind.deflection_m.abs() <= 1e-9);
}

#[test]
fn a_crosswind_drifts_the_impact_downwind() {
    let parameters = m821_parameters();
    let north = launch(66.0 * 2.085, 60.0);
    let from_west = fly(
        &parameters,
        &north,
        &Wind {
            speed_m_s: 6.0,
            from_deg: 270.0,
        },
        0.0,
    );
    assert!(
        from_west.deflection_m > 1.0,
        "wind from west must push a northward shell east: {}",
        from_west.deflection_m
    );
    let from_east = fly(
        &parameters,
        &north,
        &Wind {
            speed_m_s: 6.0,
            from_deg: 90.0,
        },
        0.0,
    );
    assert!(
        from_east.deflection_m < -1.0,
        "wind from east must push a northward shell west: {}",
        from_east.deflection_m
    );
    let east = Launch {
        azimuth_rad: std::f64::consts::FRAC_PI_2,
        ..north
    };
    let from_north = fly(
        &parameters,
        &east,
        &Wind {
            speed_m_s: 6.0,
            from_deg: 0.0,
        },
        0.0,
    );
    assert!(
        from_north.impact_position_m[1] < -1.0,
        "wind from north must push an eastward shell south"
    );
    assert!(
        from_north.deflection_m > 1.0,
        "south is to the right of an eastward shell"
    );
}

#[test]
fn a_zero_wind_influence_multiplier_ignores_the_wind() {
    let parameters = FlightParameters {
        wind_influence_multiplier: 0.0,
        ..m821_parameters()
    };
    let fired = launch(66.0 * 2.541, 65.0);
    let calm = fly(&parameters, &fired, &Wind::CALM, 0.0);
    for wind in [
        Wind {
            speed_m_s: 15.0,
            from_deg: 30.0,
        },
        Wind {
            speed_m_s: 40.0,
            from_deg: 250.0,
        },
    ] {
        assert_eq!(
            fly(&parameters, &fired, &wind, 0.0),
            calm,
            "{wind:?} moved the shell"
        );
    }
}

#[test]
fn the_flight_holds_the_engine_constants_in_single_precision() {
    let constants = super::integrator::FlightConstants::<f32>::new(
        GRAVITY_M_S2,
        0.000462,
        4.06,
        0.5,
        [3.0, -4.0, 0.0],
    );
    assert_eq!(constants.gravity_m_s2.to_bits(), 9.81_f32.to_bits());
    assert_eq!(constants.drag_per_mass, 0.000462_f32 / 4.06_f32);
    assert_eq!(constants.air_velocity_m_s, [1.5, -2.0, 0.0]);
    assert_eq!(DEFAULT_INTEGRATION_STEP_S as f32, 1.0_f32 / 30.0);
}

/// The production flight is the `f32` one: it differs from the double-precision flight of the
/// same scheme on the case lattice (the engine's own rounding, which the oracle residual of the
/// calibration tests shows the engine shares) by at most 0.02 m and 1 ms, and not bit for bit.
#[test]
fn the_production_flight_is_the_single_precision_scheme() {
    let mut differing = 0;
    for (name, parameters, speed) in case_lattice() {
        for elevation_deg in [45.0, 55.0, 65.0, 75.0, 85.0] {
            let fired = launch(speed, elevation_deg);
            let single = fly(&parameters, &fired, &Wind::CALM, 0.0);
            let double = fly_scheme(&parameters, &fired, &Wind::CALM, 0.0);
            let label = format!("{name} at {speed} m/s, {elevation_deg}°");
            assert!(
                (single.downrange_m - double.downrange_m).abs() <= 0.02,
                "{label}: {} vs {} m",
                single.downrange_m,
                double.downrange_m
            );
            assert!(
                (single.time_of_flight_s - double.time_of_flight_s).abs() <= 1e-3,
                "{label}: {} vs {} s",
                single.time_of_flight_s,
                double.time_of_flight_s
            );
            if single.downrange_m != double.downrange_m {
                differing += 1;
            }
        }
    }
    assert!(
        differing > 100,
        "only {differing} flights round differently"
    );
}

/// Export-derived spot check: the M821 at charge coefficient 1 (66 m/s) fired at 45° lands at
/// the native-table row of game build 1.8.0.13 (800 mils: 427.308 m, 9.417 s) within the
/// table's rounding and the engine's single-precision noise.
#[test]
fn m821_at_45_degrees_and_coefficient_1_matches_the_export_derived_spot_check() {
    let outcome = fly(&m821_parameters(), &launch(66.0, 45.0), &Wind::CALM, 0.0);
    assert!(
        (outcome.downrange_m - 427.308).abs() <= 0.02,
        "range {}",
        outcome.downrange_m
    );
    assert!(
        (outcome.time_of_flight_s - 9.417).abs() <= 0.002,
        "time of flight {}",
        outcome.time_of_flight_s
    );
    assert!(
        (outcome.apex_height_m - 108.73).abs() <= 0.5,
        "apex {}",
        outcome.apex_height_m
    );
}

/// One engine-oracle simulation sample of game build 1.8.0.13 (generation 6A6F008DC5395616):
/// shell mass and air drag, muzzle speed, elevation in degrees, wind speed and the direction it
/// blows from, target height, and the engine's downrange, crossrange and time of flight.
struct OracleSimulationSample {
    mass_kg: f64,
    air_drag: f64,
    muzzle_speed_m_s: f64,
    elevation_deg: f64,
    wind: Wind,
    target_height_m: f64,
    downrange_m: f64,
    crossrange_m: f64,
    time_of_flight_s: f64,
}

const ORACLE_SAMPLES: [OracleSimulationSample; 7] = [
    oracle_sample(
        4.06,
        0.000462,
        66.0,
        45.0,
        (0.0, 0.0),
        -100.0,
        [507.8890075683594, 0.0, 11.26739501953125],
    ),
    oracle_sample(
        4.06,
        0.000462,
        66.0,
        45.0,
        (0.0, 0.0),
        0.0,
        [427.30828857421875, 0.0, 9.43359375],
    ),
    oracle_sample(
        4.06,
        0.000462,
        66.0,
        45.0,
        (0.0, 0.0),
        100.0,
        [275.88519287109375, 0.0, 6.03424072265625],
    ),
    oracle_sample(
        4.06,
        0.000462,
        196.48199462890625,
        45.0,
        (5.0, 270.0),
        0.0,
        [2959.36279296875, 23.986589431762695, 26.10076904296875],
    ),
    oracle_sample(
        4.0,
        0.001488,
        242.5919952392578,
        85.0,
        (10.0, 90.0),
        -100.0,
        [422.6376037597656, -165.75375366210938, 36.56707763671875],
    ),
    oracle_sample(
        3.51,
        0.001836,
        273.5580139160156,
        65.0,
        (10.0, 180.0),
        100.0,
        [
            1798.3153076171875,
            1.5538667867076583e-05,
            31.80084228515625,
        ],
    ),
    oracle_sample(
        3.51,
        0.001836,
        273.5580139160156,
        85.0,
        (10.0, 270.0),
        0.0,
        [375.0306396484375, 197.22129821777344, 35.467529296875],
    ),
];

const fn oracle_sample(
    mass_kg: f64,
    air_drag: f64,
    muzzle_speed_m_s: f64,
    elevation_deg: f64,
    (speed_m_s, from_deg): (f64, f64),
    target_height_m: f64,
    [downrange_m, crossrange_m, time_of_flight_s]: [f64; 3],
) -> OracleSimulationSample {
    OracleSimulationSample {
        mass_kg,
        air_drag,
        muzzle_speed_m_s,
        elevation_deg,
        wind: Wind {
            speed_m_s,
            from_deg,
        },
        target_height_m,
        downrange_m,
        crossrange_m,
        time_of_flight_s,
    }
}

/// The engine's point of fall is reproduced to its single-precision noise (the largest
/// difference over all 4,185 oracle samples is 0.008 m, pinned by the calibration tests); the
/// oracle's time of flight is the end
/// of the crossing step, found by a 16-step bisection over 60 s, so it lies at most one step
/// plus one bisection interval after the crossing.
#[test]
fn engine_oracle_simulation_samples_are_reproduced_to_single_precision_noise() {
    let bisection_interval_s = 60.0 / 65536.0;
    for (index, sample) in ORACLE_SAMPLES.iter().enumerate() {
        let parameters = FlightParameters {
            mass_kg: sample.mass_kg,
            air_drag: sample.air_drag,
            ..m821_parameters()
        };
        let outcome = fly(
            &parameters,
            &launch(sample.muzzle_speed_m_s, sample.elevation_deg),
            &sample.wind,
            sample.target_height_m,
        );
        let downrange_error = (outcome.downrange_m - sample.downrange_m).abs();
        let crossrange_error = (outcome.deflection_m - sample.crossrange_m).abs();
        assert!(
            downrange_error <= 0.02 && crossrange_error <= 0.02,
            "sample {index}: ({}, {}) vs engine ({}, {})",
            outcome.downrange_m,
            outcome.deflection_m,
            sample.downrange_m,
            sample.crossrange_m
        );
        let lag_s = sample.time_of_flight_s - outcome.time_of_flight_s;
        assert!(
            (0.0..=DEFAULT_INTEGRATION_STEP_S + bisection_interval_s).contains(&lag_s),
            "sample {index}: oracle time {} vs crossing {}",
            sample.time_of_flight_s,
            outcome.time_of_flight_s
        );
    }
}
