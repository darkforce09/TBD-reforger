//! Symmetry sweeps of the firing solver over the vanilla catalog: rotating the gun, the target
//! and the wind by one azimuth rotates the aim and leaves every charge row unchanged; mirroring
//! a crosswind across the line of fire mirrors the deflection correction; raising the target
//! lowers the high-angle elevation and shortens the flight.
//!
//! The flight runs in the engine's `f32`, and a rotation changes how the muzzle velocity splits
//! into east and north components, so the flight-dependent values agree to that rounding, not
//! bit for bit: the exact quantities (rings, refusals, the recommended charge, distance, height
//! difference, the geometric azimuth and the calm-air aim) are held to 1e-9, the elevation to a
//! hundredth of a 6400-mil, and the wind-corrected aim to the miss the aim loop accepts.

use crate::data::scenario::ballistics::angular_units::normalise_azimuth_degrees;
use crate::data::scenario::ballistics::catalog::BallisticsCatalog;
use crate::data::scenario::ballistics::solver::wind_corrected_aim::AIM_MISS_TOLERANCE_M;
use crate::data::scenario::ballistics::solver::{
    ChargeSolution, FireSolution, FireSolutionRequest, MapPosition, solve_fire_solution,
};
use crate::data::scenario::ballistics::wind::Wind;

/// The committed vanilla catalog; a missing file fails the build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));

/// Gun-to-target distances every shell is solved at, metres.
const DISTANCES_M: [f64; 3] = [180.0, 420.0, 900.0];

/// Rotations applied to the gun, the target and the wind about the map origin, degrees.
const ROTATIONS_DEG: [f64; 6] = [17.3, 90.0, 133.7, 180.0, 251.9, 305.0];

/// Tolerance of the quantities a rotation or mirror leaves exactly unchanged.
const EXACT_TOLERANCE: f64 = 1e-9;

/// A hundredth of a 6400-mil in degrees: the elevation agreement across rotations and mirrors.
const ELEVATION_TOLERANCE_DEG: f64 = 0.01 * 360.0 / 6400.0;

/// Time-of-flight agreement across rotations and mirrors, seconds.
const TIME_OF_FLIGHT_TOLERANCE_S: f64 = 1e-3;

/// Apex agreement across rotations and mirrors, metres.
const APEX_TOLERANCE_M: f64 = 0.01;

/// Target heights above the gun the height sweep solves at, ascending, metres.
const HEIGHT_DIFFERENCES_M: [f64; 7] = [-150.0, -80.0, -30.0, 0.0, 30.0, 80.0, 150.0];

fn vanilla_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON).expect("the vanilla catalog decodes")
}

/// Every (weapon, shell) pairing the catalog declares.
fn pairings(catalog: &BallisticsCatalog) -> Vec<(String, String)> {
    catalog
        .weapons
        .iter()
        .flat_map(|weapon| {
            weapon
                .shell_ids
                .iter()
                .map(|shell_id| (weapon.weapon_id.clone(), shell_id.clone()))
        })
        .collect()
}

/// `(x, y)` rotated clockwise by `angle_deg` about the map origin, so an azimuth grows by it.
fn rotated(x: f64, y: f64, angle_deg: f64) -> (f64, f64) {
    let (sine, cosine) = (angle_deg.to_radians().sin(), angle_deg.to_radians().cos());
    (x * cosine + y * sine, -x * sine + y * cosine)
}

/// `distance_m` from `(x, y)` along the azimuth `bearing_deg`.
fn along(x: f64, y: f64, bearing_deg: f64, distance_m: f64) -> (f64, f64) {
    let bearing = bearing_deg.to_radians();
    (
        x + distance_m * bearing.sin(),
        y + distance_m * bearing.cos(),
    )
}

/// Signed smallest difference `a - b` between two azimuths in degrees.
fn azimuth_difference_deg(a: f64, b: f64) -> f64 {
    let difference = normalise_azimuth_degrees(a - b);
    if difference > 180.0 {
        difference - 360.0
    } else {
        difference
    }
}

/// The aim-azimuth agreement of two wind-corrected solves at `distance_m`, degrees: each aim
/// stops within [`AIM_MISS_TOLERANCE_M`] of the target, so two aims differ by at most twice
/// that miss seen from the gun.
fn wind_aim_tolerance_deg(distance_m: f64) -> f64 {
    (2.0 * AIM_MISS_TOLERANCE_M / distance_m).to_degrees()
}

fn point(x_m: f64, y_m: f64, height_m: f64) -> MapPosition {
    MapPosition { x_m, y_m, height_m }
}

fn solve(
    catalog: &BallisticsCatalog,
    (weapon_id, shell_id): (&str, &str),
    gun: MapPosition,
    target: MapPosition,
    wind: Wind,
) -> FireSolution {
    solve_fire_solution(
        catalog,
        &FireSolutionRequest {
            weapon_id,
            shell_id,
            gun,
            target,
            wind,
        },
    )
    .expect("a valid request solves")
}

fn value(row: &ChargeSolution, field: fn(&ChargeSolution) -> Option<f64>) -> f64 {
    field(row).expect("a solved row carries every value")
}

/// Records `label` when `difference` exceeds `tolerance`.
fn check(failures: &mut Vec<String>, label: &str, difference: f64, tolerance: f64) {
    if difference.abs() > tolerance || difference.is_nan() {
        failures.push(format!(
            "{label}: differs by {difference:e}, beyond {tolerance:e}"
        ));
    }
}

/// Every charge row of `base` against the same row of `turned`, a solve of the same problem
/// rotated by `rotation_deg`; answers the number of solved rows compared.
fn compare_rotated_rows(
    failures: &mut Vec<String>,
    case: &str,
    (base, turned): (&FireSolution, &FireSolution),
    rotation_deg: f64,
    aim_tolerance_deg: f64,
) -> usize {
    let mut compared = 0;
    for (base_row, turned_row) in base.charges.iter().zip(&turned.charges) {
        let label = format!("{case} rings {}", base_row.rings);
        if base_row.rings != turned_row.rings || base_row.refusal != turned_row.refusal {
            failures.push(format!(
                "{label}: rotation changes the row's rings or refusal"
            ));
            continue;
        }
        if !base_row.solves() {
            continue;
        }
        compared += 1;
        let difference = |field: fn(&ChargeSolution) -> Option<f64>| {
            value(base_row, field) - value(turned_row, field)
        };
        check(
            failures,
            &format!("{label} elevation deg"),
            difference(|row| row.elevation_deg),
            ELEVATION_TOLERANCE_DEG,
        );
        check(
            failures,
            &format!("{label} time of flight s"),
            difference(|row| row.time_of_flight_s),
            TIME_OF_FLIGHT_TOLERANCE_S,
        );
        check(
            failures,
            &format!("{label} apex m"),
            difference(|row| row.apex_m),
            APEX_TOLERANCE_M,
        );
        check(
            failures,
            &format!("{label} range correction m"),
            difference(|row| row.range_correction_m),
            2.0 * AIM_MISS_TOLERANCE_M,
        );
        let aim_turn_deg = azimuth_difference_deg(
            value(turned_row, |row| row.aim_azimuth_deg),
            value(base_row, |row| row.aim_azimuth_deg),
        );
        check(
            failures,
            &format!("{label} aim azimuth rotation deg"),
            azimuth_difference_deg(aim_turn_deg, rotation_deg),
            aim_tolerance_deg,
        );
    }
    compared
}

/// Rotates every pairing's problem under `wind`; answers the failures and the solved rows
/// compared.
fn rotation_sweep(wind: Wind) -> (Vec<String>, usize) {
    let catalog = vanilla_catalog();
    let mut failures = Vec::new();
    let mut compared = 0;
    let gun_base = (1_250.0, -2_340.0);
    for (weapon_id, shell_id) in pairings(&catalog) {
        for distance_m in DISTANCES_M {
            let target_base = along(gun_base.0, gun_base.1, 31.0, distance_m);
            let base = solve(
                &catalog,
                (&weapon_id, &shell_id),
                point(gun_base.0, gun_base.1, 40.0),
                point(target_base.0, target_base.1, 65.0),
                wind,
            );
            let aim_tolerance_deg = if wind.speed_m_s == 0.0 {
                EXACT_TOLERANCE
            } else {
                wind_aim_tolerance_deg(distance_m)
            };
            for rotation_deg in ROTATIONS_DEG {
                let case =
                    format!("{weapon_id}/{shell_id} at {distance_m} m turned {rotation_deg}°");
                let gun = rotated(gun_base.0, gun_base.1, rotation_deg);
                let target = rotated(target_base.0, target_base.1, rotation_deg);
                let turned = solve(
                    &catalog,
                    (&weapon_id, &shell_id),
                    point(gun.0, gun.1, 40.0),
                    point(target.0, target.1, 65.0),
                    Wind {
                        speed_m_s: wind.speed_m_s,
                        from_deg: normalise_azimuth_degrees(wind.from_deg + rotation_deg),
                    },
                );
                if base.recommended_rings != turned.recommended_rings
                    || base.charges.len() != turned.charges.len()
                {
                    failures.push(format!(
                        "{case}: rotation changes the charges or the recommendation"
                    ));
                }
                check(
                    &mut failures,
                    &format!("{case} distance m"),
                    base.distance_m - turned.distance_m,
                    EXACT_TOLERANCE,
                );
                check(
                    &mut failures,
                    &format!("{case} height difference m"),
                    base.height_difference_m - turned.height_difference_m,
                    EXACT_TOLERANCE,
                );
                check(
                    &mut failures,
                    &format!("{case} geometric azimuth rotation deg"),
                    azimuth_difference_deg(
                        azimuth_difference_deg(turned.azimuth_deg, base.azimuth_deg),
                        rotation_deg,
                    ),
                    EXACT_TOLERANCE,
                );
                compared += compare_rotated_rows(
                    &mut failures,
                    &case,
                    (&base, &turned),
                    rotation_deg,
                    aim_tolerance_deg,
                );
            }
        }
    }
    (failures, compared)
}

fn assert_sweep_holds(failures: &[String], compared: usize, minimum_compared: usize) {
    assert!(
        failures.is_empty(),
        "{} failures, first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
    assert!(
        compared >= minimum_compared,
        "only {compared} solved rows compared, expected at least {minimum_compared}"
    );
}

#[test]
fn rotation_in_calm_air_rotates_the_aim_and_keeps_every_charge_row() {
    let (failures, compared) = rotation_sweep(Wind::CALM);
    assert_sweep_holds(&failures, compared, 300);
}

#[test]
fn rotation_with_the_wind_rotates_the_aim_and_keeps_every_charge_row() {
    let (failures, compared) = rotation_sweep(Wind {
        speed_m_s: 8.0,
        from_deg: 64.0,
    });
    assert_sweep_holds(&failures, compared, 300);
}

#[test]
fn a_mirrored_crosswind_mirrors_the_deflection_and_keeps_the_elevation() {
    let catalog = vanilla_catalog();
    let mut failures = Vec::new();
    let mut compared = 0;
    let gun = (500.0, 700.0);
    for (weapon_id, shell_id) in pairings(&catalog) {
        for distance_m in DISTANCES_M {
            for line_deg in [0.0, 37.0, 211.0] {
                let target = along(gun.0, gun.1, line_deg, distance_m);
                let solve_from = |from_deg: f64| {
                    solve(
                        &catalog,
                        (&weapon_id, &shell_id),
                        point(gun.0, gun.1, 0.0),
                        point(target.0, target.1, 30.0),
                        Wind {
                            speed_m_s: 8.0,
                            from_deg: normalise_azimuth_degrees(from_deg),
                        },
                    )
                };
                let from_right = solve_from(line_deg + 90.0);
                let from_left = solve_from(line_deg - 90.0);
                let tolerance_deg = wind_aim_tolerance_deg(distance_m);
                for (right, left) in from_right.charges.iter().zip(&from_left.charges) {
                    let label = format!(
                        "{weapon_id}/{shell_id} at {distance_m} m on {line_deg}° rings {}",
                        right.rings
                    );
                    if right.refusal != left.refusal {
                        failures.push(format!("{label}: the mirror changes the refusal"));
                        continue;
                    }
                    if !right.solves() {
                        continue;
                    }
                    compared += 1;
                    let difference = |field: fn(&ChargeSolution) -> Option<f64>| {
                        value(right, field) - value(left, field)
                    };
                    check(
                        &mut failures,
                        &format!("{label} elevation deg"),
                        difference(|row| row.elevation_deg),
                        ELEVATION_TOLERANCE_DEG,
                    );
                    check(
                        &mut failures,
                        &format!("{label} time of flight s"),
                        difference(|row| row.time_of_flight_s),
                        TIME_OF_FLIGHT_TOLERANCE_S,
                    );
                    check(
                        &mut failures,
                        &format!("{label} range correction m"),
                        difference(|row| row.range_correction_m),
                        2.0 * AIM_MISS_TOLERANCE_M,
                    );
                    let right_offset_deg =
                        azimuth_difference_deg(value(right, |row| row.aim_azimuth_deg), line_deg);
                    let left_offset_deg =
                        azimuth_difference_deg(value(left, |row| row.aim_azimuth_deg), line_deg);
                    check(
                        &mut failures,
                        &format!("{label} mirrored aim offset deg"),
                        right_offset_deg + left_offset_deg,
                        tolerance_deg,
                    );
                    let mils_per_deg = f64::from(from_right.mils_per_circle) / 360.0;
                    check(
                        &mut failures,
                        &format!("{label} mirrored deflection correction mils"),
                        value(right, |row| row.deflection_correction_mils)
                            + value(left, |row| row.deflection_correction_mils),
                        tolerance_deg * mils_per_deg,
                    );
                    // Wind from the right drifts the shell left, so the aim lays right.
                    if right_offset_deg <= tolerance_deg {
                        failures.push(format!(
                            "{label}: a wind from the right lays the aim {right_offset_deg}° (not right of the line)"
                        ));
                    }
                }
            }
        }
    }
    assert_sweep_holds(&failures, compared, 150);
}

#[test]
fn raising_the_target_lowers_the_elevation_and_shortens_the_flight() {
    let catalog = vanilla_catalog();
    let mut failures = Vec::new();
    let (mut compared, mut above, mut below) = (0, 0, 0);
    for (weapon_id, shell_id) in pairings(&catalog) {
        for distance_m in DISTANCES_M {
            let target = along(0.0, 0.0, 123.0, distance_m);
            let solutions: Vec<(f64, FireSolution)> = HEIGHT_DIFFERENCES_M
                .iter()
                .map(|&height_m| {
                    let solution = solve(
                        &catalog,
                        (&weapon_id, &shell_id),
                        point(0.0, 0.0, 0.0),
                        point(target.0, target.1, height_m),
                        Wind::CALM,
                    );
                    (height_m, solution)
                })
                .collect();
            for charge_index in 0..solutions[0].1.charges.len() {
                let solved: Vec<(f64, &ChargeSolution)> = solutions
                    .iter()
                    .map(|(height_m, solution)| (*height_m, &solution.charges[charge_index]))
                    .filter(|(_, row)| row.solves())
                    .collect();
                for pair in solved.windows(2) {
                    let ((lower_m, lower), (higher_m, higher)) = (pair[0], pair[1]);
                    let label = format!(
                        "{weapon_id}/{shell_id} at {distance_m} m rings {} from {lower_m} m to {higher_m} m",
                        lower.rings
                    );
                    compared += 1;
                    if higher_m > 0.0 && lower_m >= 0.0 {
                        above += 1;
                    }
                    if lower_m < 0.0 && higher_m <= 0.0 {
                        below += 1;
                    }
                    let elevation_drop = value(lower, |row| row.elevation_deg)
                        - value(higher, |row| row.elevation_deg);
                    let flight_shortening = value(lower, |row| row.time_of_flight_s)
                        - value(higher, |row| row.time_of_flight_s);
                    if elevation_drop <= ELEVATION_TOLERANCE_DEG {
                        failures.push(format!(
                            "{label}: elevation drops by only {elevation_drop}°"
                        ));
                    }
                    if flight_shortening <= TIME_OF_FLIGHT_TOLERANCE_S {
                        failures.push(format!(
                            "{label}: flight shortens by only {flight_shortening} s"
                        ));
                    }
                }
            }
        }
    }
    assert_sweep_holds(&failures, compared, 200);
    assert!(
        above > 0 && below > 0,
        "heights above ({above}) and below ({below}) the gun are both compared"
    );
}
