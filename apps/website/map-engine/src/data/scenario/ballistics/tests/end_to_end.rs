//! End to end from the committed vanilla catalog: for every weapon and every shell it fires, a
//! two-gun fire mission in wind onto a raised target, with a burst height for a time-fuzed
//! shell and a terrain profile under the lead gun's line, is solved through
//! [`solve_fire_mission`] into a whole solution: each gun's recommended charge, the lead gun's
//! dispersion, fuze and crest clearance, and the catalog and solver provenance. Each gun's
//! recommended charge, flown again at its aim azimuth and elevation, lands on the target, and
//! the solution is deterministic and survives a JSON round trip.

use crate::data::scenario::ballistics::catalog::{
    BallisticsCatalog, flight_parameters, muzzle_speed_m_s,
};
use crate::data::scenario::ballistics::crest_clearance::{TerrainProfile, TerrainSample};
use crate::data::scenario::ballistics::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionSolution,
    FireMissionWind, HeightSource, SOLVER_REVISION, solve_fire_mission,
};
use crate::data::scenario::ballistics::flight_model::{Launch, PathRecording, fly_to_height};
use crate::data::scenario::ballistics::wind::Wind;

/// The committed vanilla catalog; a missing file fails the build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));

/// The mission's wind.
const WIND: FireMissionWind = FireMissionWind {
    speed_m_s: 6.0,
    from_deg: 250.0,
};

/// Burst height above the target for a time-fuzed shell, metres.
const BURST_HEIGHT_M: f64 = 150.0;

/// Largest horizontal miss of a re-flown recommended charge, metres: the aim loop stops within
/// 0.01 m and the elevation leaves the solution in degrees.
const LANDING_TOLERANCE_M: f64 = 0.1;

fn vanilla_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON).expect("the vanilla catalog decodes")
}

fn gun(label: &str, x: f64, y: f64, height_m: f64) -> FireMissionGunPosition {
    FireMissionGunPosition {
        label: label.to_owned(),
        x,
        y,
        height_m,
        height_source: HeightSource::Dem,
    }
}

/// Two guns about 350 m south-west of a target 55 m above the lead gun, in a 6 m/s wind, with
/// a terrain rise under the lead gun's line.
fn mission(catalog: &BallisticsCatalog, weapon_id: &str, shell_id: &str) -> FireMissionInputs {
    let fuzed = catalog
        .shell(shell_id)
        .expect("the shell is in the catalog")
        .time_fuze
        .is_some();
    FireMissionInputs {
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        weapon_id: weapon_id.to_owned(),
        shell_id: shell_id.to_owned(),
        charge_rings: None,
        target: FireMissionPoint {
            x: 4_225.0,
            y: 6_268.0,
            height_m: 95.0,
            height_source: HeightSource::Manual,
        },
        guns: vec![
            gun("Gun 1", 4_000.0, 6_000.0, 40.0),
            gun("Gun 2", 4_030.0, 5_985.0, 42.0),
        ],
        wind: Some(WIND),
        burst_height_m: fuzed.then_some(BURST_HEIGHT_M),
        crest_profile: Some(TerrainProfile {
            samples: vec![
                TerrainSample {
                    downrange_m: 60.0,
                    height_m: 48.0,
                },
                TerrainSample {
                    downrange_m: 180.0,
                    height_m: 71.0,
                },
                TerrainSample {
                    downrange_m: 300.0,
                    height_m: 88.0,
                },
            ],
        }),
    }
}

/// Every problem with `solution` of `inputs`, as text.
fn problems(
    catalog: &BallisticsCatalog,
    inputs: &FireMissionInputs,
    solution: &FireMissionSolution,
) -> Vec<String> {
    let mut found = Vec::new();
    let weapon = catalog
        .weapon(&inputs.weapon_id)
        .expect("the weapon is in the catalog");
    let shell = catalog
        .shell(&inputs.shell_id)
        .expect("the shell is in the catalog");
    if solution.catalog_id != catalog.catalog_id
        || solution.catalog_version != catalog.catalog_version
        || solution.solver_revision != SOLVER_REVISION
    {
        found.push("provenance is not the pinned catalog and solver revision".to_owned());
    }
    if solution.guns.len() != inputs.guns.len() {
        found.push(format!(
            "{} gun solutions for {} guns",
            solution.guns.len(),
            inputs.guns.len()
        ));
    }
    for (gun_solution, gun) in solution.guns.iter().zip(&inputs.guns) {
        let Some(charge) = gun_solution.recommended_charge().filter(|row| row.solves()) else {
            found.push(format!("{} has no recommended charge", gun.label));
            continue;
        };
        if gun_solution.dispersion.is_none() {
            found.push(format!("{} has no dispersion", gun.label));
        }
        let (Some(elevation_deg), Some(aim_azimuth_deg)) =
            (charge.elevation_deg, charge.aim_azimuth_deg)
        else {
            found.push(format!("{} recommends a row without an aim", gun.label));
            continue;
        };
        let launch = Launch {
            muzzle_speed_m_s: muzzle_speed_m_s(
                weapon,
                shell,
                shell
                    .charge(charge.rings)
                    .expect("a solved row names a shell charge"),
            ),
            elevation_rad: elevation_deg.to_radians(),
            azimuth_rad: aim_azimuth_deg.to_radians(),
        };
        let wind = Wind {
            speed_m_s: WIND.speed_m_s,
            from_deg: WIND.from_deg,
        };
        match fly_to_height(
            &flight_parameters(catalog.gravity_m_s2, shell),
            &launch,
            &wind,
            inputs.target.height_m - gun.height_m,
            PathRecording::Discard,
        ) {
            Ok(flight) => {
                let miss_m = libm::hypot(
                    gun.x + flight.impact_position_m[0] - inputs.target.x,
                    gun.y + flight.impact_position_m[1] - inputs.target.y,
                );
                if miss_m > LANDING_TOLERANCE_M {
                    found.push(format!(
                        "{} charge {} lands {miss_m:.3} m off the target",
                        gun.label, charge.rings
                    ));
                }
            }
            Err(error) => found.push(format!(
                "{} charge {} does not fly: {error}",
                gun.label, charge.rings
            )),
        }
    }
    if solution.dispersion.is_none() {
        found.push("the lead gun has no dispersion".to_owned());
    }
    match (&shell.time_fuze, &solution.fuze) {
        (None, None) => {}
        (Some(window), Some(fuze)) => match fuze.time_s {
            Some(time_s)
                if (window.min_s..=window.max_s).contains(&time_s) && fuze.burst_aim.is_some() => {}
            _ => found.push(format!("the fuze does not set: {fuze:?}")),
        },
        (window, fuze) => found.push(format!("fuze {fuze:?} for the shell's window {window:?}")),
    }
    match &solution.crest {
        Some(crest) if !crest.is_blocked() && crest.min_clearance_m.is_finite() => {}
        crest => found.push(format!(
            "the crest clearance is missing or blocked: {crest:?}"
        )),
    }
    found
}

#[test]
fn every_weapon_and_shell_of_the_vanilla_catalog_solves_a_whole_fire_mission() {
    let catalog = vanilla_catalog();
    let mut failures = Vec::new();
    let mut solved_shells = Vec::new();
    for weapon in &catalog.weapons {
        for shell_id in &weapon.shell_ids {
            let inputs = mission(&catalog, &weapon.weapon_id, shell_id);
            let case = format!("{}/{shell_id}", weapon.weapon_id);
            match solve_fire_mission(&catalog, &inputs) {
                Ok(solution) => {
                    let found = problems(&catalog, &inputs, &solution);
                    failures.extend(
                        found
                            .into_iter()
                            .map(|problem| format!("{case}: {problem}")),
                    );
                    solved_shells.push(shell_id.clone());
                }
                Err(refusal) => failures.push(format!("{case}: refused: {refusal}")),
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    let mut catalog_shells: Vec<String> = catalog
        .shells
        .iter()
        .map(|shell| shell.shell_id.clone())
        .collect();
    catalog_shells.sort_unstable();
    solved_shells.sort_unstable();
    assert_eq!(
        solved_shells, catalog_shells,
        "every catalog shell is fired by a weapon and solved"
    );
}

#[test]
fn a_vanilla_fire_mission_is_deterministic_and_survives_a_json_round_trip() {
    let catalog = vanilla_catalog();
    for weapon in &catalog.weapons {
        for shell_id in &weapon.shell_ids {
            let inputs = mission(&catalog, &weapon.weapon_id, shell_id);
            let first = solve_fire_mission(&catalog, &inputs).expect("the mission solves");
            let second = solve_fire_mission(&catalog, &inputs).expect("the mission solves");
            assert_eq!(
                first, second,
                "{}/{shell_id} solves differently twice",
                weapon.weapon_id
            );
            let json = serde_json::to_string(&first).expect("the solution serialises");
            let decoded: FireMissionSolution =
                serde_json::from_str(&json).expect("the solution decodes");
            assert_eq!(
                decoded, first,
                "{}/{shell_id} changes through JSON",
                weapon.weapon_id
            );
            let inputs_json = serde_json::to_string(&inputs).expect("the inputs serialise");
            let decoded_inputs: FireMissionInputs =
                serde_json::from_str(&inputs_json).expect("the inputs decode");
            assert_eq!(
                solve_fire_mission(&catalog, &decoded_inputs).expect("the decoded mission solves"),
                first,
                "{}/{shell_id} solves differently from its decoded inputs",
                weapon.weapon_id
            );
        }
    }
}
