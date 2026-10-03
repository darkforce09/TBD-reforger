//! Tests of the client/server mismatch rule: identical solutions agree, a skew just inside a
//! tolerance agrees and one just outside is flagged with both values, azimuths compare the
//! short way round, and provenance, structure and presence differences are flagged.

use super::*;
use crate::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionWind, HeightSource,
    solve_fire_mission,
};
use ballistics_model::catalog::BallisticsCatalog;
use ballistics_model::ids::{CatalogId, ShellId, WeaponId};
use ballistics_solver::crest_clearance::{TerrainProfile, TerrainSample};

const MILS_PER_CIRCLE: u32 = 6400;

fn gun(label: &str, x: f64, y: f64, height_m: f64) -> FireMissionGunPosition {
    FireMissionGunPosition {
        label: label.to_owned(),
        x,
        y,
        height_m,
        height_source: HeightSource::Dem,
    }
}

/// A three-gun illumination mission in wind with its fuze and crest (the `test-mortars`
/// sample catalog, weapon convention 6400 mils).
fn server_solution() -> FireMissionSolution {
    let catalog = BallisticsCatalog::from_json_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/ballistics/minimal_catalog.json"
    )))
    .expect("the sample catalog decodes");
    let inputs = FireMissionInputs {
        catalog_id: CatalogId::new("test-mortars"),
        catalog_version: 3,
        weapon_id: WeaponId::new("m252"),
        shell_id: ShellId::new("m853-illumination"),
        charge_rings: Some(1),
        target: FireMissionPoint {
            x: 240.0,
            y: 180.0,
            height_m: 25.0,
            height_source: HeightSource::Manual,
        },
        guns: vec![
            gun("gun-1", 0.0, 0.0, 10.0),
            gun("gun-2", 30.0, -20.0, 12.0),
            gun("gun-3", -25.0, 15.0, 8.0),
        ],
        wind: Some(FireMissionWind {
            speed_m_s: 7.0,
            from_deg: 250.0,
        }),
        burst_height_m: Some(150.0),
        crest_profile: Some(TerrainProfile {
            samples: vec![TerrainSample {
                downrange_m: 120.0,
                height_m: 20.0,
            }],
        }),
    };
    let solution = solve_fire_mission(&catalog, &inputs).expect("the fire mission solves");
    assert!(solution.fuze.and_then(|fuze| fuze.burst_aim).is_some());
    solution
}

/// The index of gun 1's charge row of 1 ring, which solves.
fn solved_row(solution: &FireMissionSolution) -> usize {
    solution.guns[1]
        .charges
        .iter()
        .position(|row| row.rings == 1 && row.solves())
        .expect("gun 1 solves charge 1")
}

fn compare(client: &FireMissionSolution, server: &FireMissionSolution) -> SolutionComparison {
    compare_solutions(client, server, MILS_PER_CIRCLE)
}

fn gun_row(rings: u32) -> ComparedRow {
    ComparedRow::GunCharge {
        gun_index: 1,
        rings,
    }
}

#[test]
fn identical_solutions_agree() {
    let server = server_solution();
    let comparison = compare(&server.clone(), &server);
    assert!(comparison.agrees(), "{comparison:#?}");
}

#[test]
fn an_aim_azimuth_skew_of_one_point_zero_one_mils_is_flagged_and_zero_point_nine_nine_is_not() {
    let server = server_solution();
    let row = solved_row(&server);
    let skewed = |skew_mils: f64| {
        let mut client = server.clone();
        let aim = &mut client.guns[1].charges[row].aim_azimuth_mils;
        *aim = aim.map(|mils| mils + skew_mils);
        client
    };
    assert!(compare(&skewed(0.99), &server).agrees());
    assert!(compare(&skewed(-0.99), &server).agrees());
    let refused = compare(&skewed(1.01), &server);
    let rings = server.guns[1].charges[row].rings;
    assert_eq!(refused.mismatches.len(), 1, "{refused:#?}");
    assert!(matches!(
        &refused.mismatches[0],
        SolutionMismatch::Value {
            row: compared,
            quantity: ComparedQuantity::AimAzimuthMils,
            client: Some(_),
            server: Some(_),
            tolerance,
        } if *compared == gun_row(rings) && *tolerance == ANGLE_TOLERANCE_MILS
    ));
    assert!(!compare(&skewed(-1.01), &server).agrees());
}

#[test]
fn an_elevation_skew_is_held_to_one_mil_and_a_time_of_flight_skew_to_a_tenth_of_a_second() {
    let server = server_solution();
    let row = solved_row(&server);
    let with = |elevation_skew: f64, time_skew: f64| {
        let mut client = server.clone();
        let charge = &mut client.guns[1].charges[row];
        charge.elevation_mils = charge.elevation_mils.map(|mils| mils + elevation_skew);
        charge.time_of_flight_s = charge.time_of_flight_s.map(|time| time + time_skew);
        client
    };
    assert!(compare(&with(0.99, 0.09), &server).agrees());
    let elevation = compare(&with(1.01, 0.0), &server);
    assert!(matches!(
        elevation.mismatches.as_slice(),
        [SolutionMismatch::Value {
            quantity: ComparedQuantity::ElevationMils,
            ..
        }]
    ));
    let time = compare(&with(0.0, 0.11), &server);
    assert!(matches!(
        time.mismatches.as_slice(),
        [SolutionMismatch::Value {
            quantity: ComparedQuantity::TimeOfFlightS,
            tolerance,
            ..
        }] if *tolerance == TIME_TOLERANCE_S
    ));
}

#[test]
fn aim_azimuths_compare_the_short_way_round_the_circle() {
    let mut server = server_solution();
    let row = solved_row(&server);
    let mut client = server.clone();
    server.guns[1].charges[row].aim_azimuth_mils = Some(0.3);
    client.guns[1].charges[row].aim_azimuth_mils = Some(6_399.8);
    assert!(compare(&client, &server).agrees());
    client.guns[1].charges[row].aim_azimuth_mils = Some(6_399.2);
    assert!(!compare(&client, &server).agrees());
}

#[test]
fn the_fuze_burst_aim_is_compared_like_a_charge_row() {
    let server = server_solution();
    let skewed = |elevation_skew: f64, fuze_time_skew: f64| {
        let mut client = server.clone();
        let fuze = client.fuze.as_mut().expect("fuze");
        fuze.time_s = fuze.time_s.map(|time| time + fuze_time_skew);
        let aim = fuze.burst_aim.as_mut().expect("burst aim");
        aim.elevation_mils += elevation_skew;
        client
    };
    assert!(compare(&skewed(0.99, 0.09), &server).agrees());
    assert!(matches!(
        compare(&skewed(1.01, 0.0), &server).mismatches.as_slice(),
        [SolutionMismatch::Value {
            row: ComparedRow::Fuze,
            quantity: ComparedQuantity::ElevationMils,
            ..
        }]
    ));
    assert!(matches!(
        compare(&skewed(0.0, 0.11), &server).mismatches.as_slice(),
        [SolutionMismatch::Value {
            row: ComparedRow::Fuze,
            quantity: ComparedQuantity::FuzeTimeS,
            ..
        }]
    ));
    let mut no_fuze = server.clone();
    no_fuze.fuze = None;
    assert!(matches!(
        compare(&no_fuze, &server).mismatches.as_slice(),
        [SolutionMismatch::Fuze {
            client_burst_rings: None,
            server_burst_rings: Some(1),
            ..
        }]
    ));
}

#[test]
fn provenance_differences_are_flagged() {
    let server = server_solution();
    let mut revised = server.clone();
    revised.solver_revision = "game-ballistics-0".to_owned();
    assert!(matches!(
        compare(&revised, &server).mismatches.as_slice(),
        [SolutionMismatch::Provenance {
            field: ProvenanceField::SolverRevision,
            ..
        }]
    ));
    let mut other_catalog = server.clone();
    other_catalog.catalog_id = CatalogId::new("other");
    other_catalog.catalog_version = 2;
    let fields: Vec<_> = compare(&other_catalog, &server)
        .mismatches
        .into_iter()
        .map(|mismatch| match mismatch {
            SolutionMismatch::Provenance { field, .. } => field,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        fields,
        [ProvenanceField::CatalogId, ProvenanceField::CatalogVersion]
    );
}

#[test]
fn structural_and_presence_differences_are_flagged() {
    let server = server_solution();
    let row = solved_row(&server);

    let mut fewer_guns = server.clone();
    fewer_guns.guns.pop();
    assert!(
        compare(&fewer_guns, &server)
            .mismatches
            .contains(&SolutionMismatch::GunCount {
                client: 2,
                server: 3,
            })
    );

    let mut missing_value = server.clone();
    missing_value.guns[1].charges[row].elevation_mils = None;
    assert!(!compare(&missing_value, &server).agrees());

    let mut not_a_number = server.clone();
    not_a_number.guns[1].charges[row].time_of_flight_s = Some(f64::NAN);
    assert!(!compare(&not_a_number, &server).agrees());

    let mut refused = server.clone();
    refused.guns[1].charges[row].refusal = Some(SolutionRefusal::OutOfRange);
    assert!(matches!(
        compare(&refused, &server).mismatches.as_slice(),
        [SolutionMismatch::ChargeRefusal { .. }]
    ));

    let mut recommended = server.clone();
    recommended.guns[1].recommended_rings = Some(7);
    assert!(matches!(
        compare(&recommended, &server).mismatches.as_slice(),
        [SolutionMismatch::RecommendedCharge { gun_index: 1, .. }]
    ));

    let mut rows = server.clone();
    rows.guns[1].charges.pop();
    assert!(matches!(
        compare(&rows, &server).mismatches.as_slice(),
        [SolutionMismatch::ChargeRings { gun_index: 1, .. }]
    ));

    assert!(matches!(
        compare_solutions(&server, &server, 6000)
            .mismatches
            .as_slice(),
        [SolutionMismatch::MilsPerCircle { .. }, ..]
    ));
}

#[test]
fn a_comparison_serialises_both_values_for_the_mismatch_answer() {
    let server = server_solution();
    let mut client = server.clone();
    client.solver_revision = "game-ballistics-0".to_owned();
    let document = serde_json::to_value(compare(&client, &server)).expect("serialises");
    assert_eq!(
        document,
        serde_json::json!({"mismatches": [{
            "kind": "provenance",
            "field": "solver_revision",
            "client": "game-ballistics-0",
            "server": server.solver_revision,
        }]})
    );
}
