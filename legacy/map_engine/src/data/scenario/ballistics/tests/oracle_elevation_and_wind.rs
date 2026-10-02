//! The firing solver run backwards over the engine oracle: every committed simulation sample
//! with a target height or a wind other than zero is inverted, its point of fall handed to the
//! solver as the target.
//!
//! A sample launched on the high-angle branch (the model's downrange falls as the elevation
//! rises through the launch elevation) must invert to the oracle's launch: the solver's
//! elevation within 1 mil of the 6400 convention, its wind-corrected aim azimuth within 1 mil
//! of the launch azimuth, and its time of flight within 0.1 s. The oracle fires at exactly the
//! weapon's elevation limits, where the model's inverse lies within a hair on either side of the
//! limit, so the inversion searches the limits widened by the 1-mil tolerance. A sample on the
//! low-angle branch (a target 100 m up, fired at 45° or 55°) has no low-angle answer from a
//! high-angle solver: it must solve to the steeper twin that lands on the same point, never to a
//! flatter elevation or a shorter flight. One test per shell keeps each sweep a few hundred
//! solves.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::data::scenario::ballistics::angular_units::normalise_azimuth_radians;
use crate::data::scenario::ballistics::calibration::{CalibrationBundle, OracleSampleKind};
use crate::data::scenario::ballistics::catalog::{
    BallisticsCatalog, flight_parameters, muzzle_speed_m_s,
};
use crate::data::scenario::ballistics::flight_model::{Launch, PathRecording, fly_to_height};
use crate::data::scenario::ballistics::solver::{ChargeProblem, solve_charge_elevation};
use crate::data::scenario::ballistics::wind::Wind;

/// The committed vanilla catalog; a missing file fails the build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));

/// The committed calibration bundle of that catalog; a missing file fails the build.
const CALIBRATION_BUNDLE_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json"
));

/// One mil of the 6400 convention, radians.
const ONE_MIL_6400_RAD: f64 = core::f64::consts::TAU / 6400.0;

/// Time-of-flight agreement with the oracle, seconds.
const TIME_OF_FLIGHT_TOLERANCE_S: f64 = 0.1;

/// The launch of one simulation sample.
#[derive(Debug, Clone, Copy, Deserialize)]
struct SimulationInputs {
    rings: u32,
    elevation_deg: f64,
    azimuth_deg: f64,
    wind_speed_m_s: f64,
    wind_from_deg: f64,
    target_height_m: f64,
}

/// Where the engine's shell fell through the target height.
#[derive(Debug, Clone, Copy, Deserialize)]
struct SimulationOutputs {
    downrange_m: f64,
    crossrange_m: f64,
    time_of_flight_s: f64,
    reached_target_height: bool,
}

fn catalog() -> &'static BallisticsCatalog {
    static CATALOG: OnceLock<BallisticsCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON)
            .expect("the vanilla catalog decodes")
    })
}

fn bundle() -> &'static CalibrationBundle {
    static BUNDLE: OnceLock<CalibrationBundle> = OnceLock::new();
    BUNDLE.get_or_init(|| {
        CalibrationBundle::from_json_slice(CALIBRATION_BUNDLE_JSON)
            .expect("the committed calibration bundle decodes")
    })
}

/// Signed smallest difference `a - b` between two angles in radians.
fn angle_difference_rad(a: f64, b: f64) -> f64 {
    libm::remainder(a - b, core::f64::consts::TAU)
}

/// What one shell's inversion found.
#[derive(Debug, Default)]
struct Inversion {
    failures: Vec<String>,
    high_branch: usize,
    low_branch: usize,
}

/// Inverts every selected simulation sample of `shell_id`.
fn invert_shell_samples(shell_id: &str) -> Inversion {
    let catalog = catalog();
    let weapon = catalog
        .weapons
        .iter()
        .find(|weapon| weapon.fires_shell(shell_id))
        .expect("a catalog weapon fires the shell");
    let shell = catalog
        .shell(shell_id)
        .expect("the shell is in the catalog");
    let parameters = flight_parameters(catalog.gravity_m_s2, shell);
    let (lowest_rad, highest_rad) = weapon.elevation_limits_rad();
    let mut inversion = Inversion::default();
    for (index, sample) in bundle().oracle_samples.iter().enumerate() {
        if sample.kind != OracleSampleKind::Simulation || sample.shell_id != shell_id {
            continue;
        }
        let inputs = SimulationInputs::deserialize(&sample.inputs)
            .expect("a simulation sample carries its launch");
        let outputs = SimulationOutputs::deserialize(&sample.outputs)
            .expect("a simulation sample carries its point of fall");
        if inputs.target_height_m == 0.0 && inputs.wind_speed_m_s == 0.0 {
            continue;
        }
        let case = format!("simulation/{shell_id}/{}/{index}", sample.init_speed_coef);
        let failures = &mut inversion.failures;
        if !outputs.reached_target_height {
            failures.push(format!(
                "{case}: the engine never lands, there is no target to invert"
            ));
            continue;
        }
        let Some(charge) = shell.charge(inputs.rings) else {
            failures.push(format!(
                "{case}: the shell has no charge of {} rings",
                inputs.rings
            ));
            continue;
        };
        if charge.init_speed_coef != sample.init_speed_coef {
            failures.push(format!(
                "{case}: rings {} fire coefficient {}",
                inputs.rings, charge.init_speed_coef
            ));
            continue;
        }
        let launch_elevation_rad = inputs.elevation_deg.to_radians();
        let launch_azimuth_rad = inputs.azimuth_deg.to_radians();
        let (sine, cosine) = (libm::sin(launch_azimuth_rad), libm::cos(launch_azimuth_rad));
        let east_m = outputs.downrange_m * sine + outputs.crossrange_m * cosine;
        let north_m = outputs.downrange_m * cosine - outputs.crossrange_m * sine;
        let problem = ChargeProblem {
            flight_parameters: parameters,
            muzzle_speed_m_s: muzzle_speed_m_s(weapon, shell, charge),
            elevation_limits_rad: (
                lowest_rad - ONE_MIL_6400_RAD,
                highest_rad + ONE_MIL_6400_RAD,
            ),
            azimuth_rad: normalise_azimuth_radians(libm::atan2(east_m, north_m)),
            distance_m: libm::hypot(east_m, north_m),
            height_difference_m: inputs.target_height_m,
            wind: Wind {
                speed_m_s: inputs.wind_speed_m_s,
                from_deg: inputs.wind_from_deg,
            },
        };
        let downrange_at = |elevation_rad: f64| {
            let launch = Launch {
                muzzle_speed_m_s: problem.muzzle_speed_m_s,
                elevation_rad,
                azimuth_rad: launch_azimuth_rad,
            };
            fly_to_height(
                &parameters,
                &launch,
                &problem.wind,
                inputs.target_height_m,
                PathRecording::Discard,
            )
            .map_or(f64::NAN, |flight| flight.downrange_m)
        };
        let on_high_branch = downrange_at(launch_elevation_rad + ONE_MIL_6400_RAD)
            < downrange_at(launch_elevation_rad - ONE_MIL_6400_RAD);
        let solved = match solve_charge_elevation(&problem) {
            Ok(solved) => solved,
            Err(refusal) => {
                failures.push(format!("{case}: the solver refuses with {refusal:?}"));
                continue;
            }
        };
        let elevation_error_mils = (solved.elevation_rad - launch_elevation_rad) / ONE_MIL_6400_RAD;
        let time_of_flight_error_s = solved.time_of_flight_s - outputs.time_of_flight_s;
        if !on_high_branch {
            inversion.low_branch += 1;
            if elevation_error_mils < -1.0 || time_of_flight_error_s < -TIME_OF_FLIGHT_TOLERANCE_S {
                failures.push(format!(
                    "{case}: a low-angle launch at {}° solves flatter, {:.4}° in {:.4} s",
                    inputs.elevation_deg,
                    solved.elevation_rad.to_degrees(),
                    solved.time_of_flight_s
                ));
            }
            continue;
        }
        inversion.high_branch += 1;
        let aim_error_mils =
            angle_difference_rad(solved.aim_azimuth_rad, launch_azimuth_rad) / ONE_MIL_6400_RAD;
        if elevation_error_mils.abs() > 1.0 {
            failures.push(format!(
                "{case}: elevation {:.4}° against the oracle's {}° ({elevation_error_mils:.3} mil)",
                solved.elevation_rad.to_degrees(),
                inputs.elevation_deg
            ));
        }
        if aim_error_mils.abs() > 1.0 {
            failures.push(format!(
                "{case}: aim azimuth {:.4}° against the oracle's {}° ({aim_error_mils:.3} mil)",
                solved.aim_azimuth_rad.to_degrees(),
                inputs.azimuth_deg
            ));
        }
        if time_of_flight_error_s.abs() > TIME_OF_FLIGHT_TOLERANCE_S {
            failures.push(format!(
                "{case}: time of flight {:.4} s against the oracle's {} s",
                solved.time_of_flight_s, outputs.time_of_flight_s
            ));
        }
    }
    inversion
}

/// Every selected sample of `shell_id` inverts within tolerance, and the shell has samples on
/// the high-angle branch.
fn assert_shell_inverts(shell_id: &str) {
    let inversion = invert_shell_samples(shell_id);
    assert!(
        inversion.failures.is_empty(),
        "{} of {} samples fail, first: {:#?}",
        inversion.failures.len(),
        inversion.high_branch + inversion.low_branch,
        &inversion.failures[..inversion.failures.len().min(20)]
    );
    assert!(
        inversion.high_branch > 0,
        "shell {shell_id} has no high-angle sample with a height or a wind"
    );
}

#[test]
fn oracle_samples_of_m821_invert_to_the_launch_elevation() {
    assert_shell_inverts("m821");
}

#[test]
fn oracle_samples_of_m819_invert_to_the_launch_elevation() {
    assert_shell_inverts("m819");
}

#[test]
fn oracle_samples_of_m853a1_invert_to_the_launch_elevation() {
    assert_shell_inverts("m853a1");
}

#[test]
fn oracle_samples_of_m879_invert_to_the_launch_elevation() {
    assert_shell_inverts("m879");
}

#[test]
fn oracle_samples_of_o832du_invert_to_the_launch_elevation() {
    assert_shell_inverts("o832du");
}

#[test]
fn oracle_samples_of_d832du_invert_to_the_launch_elevation() {
    assert_shell_inverts("d832du");
}

#[test]
fn oracle_samples_of_s832s_invert_to_the_launch_elevation() {
    assert_shell_inverts("s832s");
}

#[test]
fn the_inverted_shells_are_every_catalog_shell_and_every_selected_sample() {
    let tested = [
        "m821", "m819", "m853a1", "m879", "o832du", "d832du", "s832s",
    ];
    let mut catalog_shells: Vec<&str> = catalog()
        .shells
        .iter()
        .map(|shell| shell.shell_id.as_str())
        .collect();
    catalog_shells.sort_unstable();
    let mut tested_sorted = tested.to_vec();
    tested_sorted.sort_unstable();
    assert_eq!(catalog_shells, tested_sorted);
    let selected = bundle()
        .oracle_samples
        .iter()
        .filter(|sample| sample.kind == OracleSampleKind::Simulation)
        .filter(|sample| {
            let inputs = SimulationInputs::deserialize(&sample.inputs)
                .expect("a simulation sample carries its launch");
            inputs.target_height_m != 0.0 || inputs.wind_speed_m_s != 0.0
        })
        .filter(|sample| !tested.contains(&sample.shell_id.as_str()))
        .count();
    assert_eq!(selected, 0, "samples of a shell no test inverts");
}
