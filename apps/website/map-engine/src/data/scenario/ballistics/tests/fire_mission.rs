//! Tests of the fire-mission assembler: schema parity of the solution and the inputs,
//! determinism, a three-gun illumination battery in wind with its fuze and crest, the fired
//! charge, the fuze charge search on the vanilla walkthrough, and the refusals.

use serde_json::{Value, json};

use super::*;
use crate::data::scenario::ballistics::crest_clearance::TerrainSample;
use crate::data::scenario::ballistics::fuze::FuzeError;

/// The contract the solution and the inputs project; a missing file fails the build.
const FIRE_MISSION_SCHEMA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/definitions/fire-mission.schema.json"
));

/// The hand-written test catalog (`test-mortars` v3: `m252` fires `m821-he` rings 0/1/2 and
/// the time-fuzed `m853-illumination` rings 0/1).
fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("../catalog/tests/minimal_catalog.json"))
        .expect("the sample catalog decodes")
}

/// A validator of `definition` in the fire-mission schema.
fn validator(definition: &str) -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(FIRE_MISSION_SCHEMA_JSON).expect("the fire-mission schema is JSON");
    let rooted = json!({
        "$schema": schema["$schema"],
        "$ref": format!("#/definitions/{definition}"),
        "definitions": schema["definitions"],
    });
    jsonschema::validator_for(&rooted).expect("the fire-mission schema compiles")
}

fn schema_errors(validator: &jsonschema::Validator, document: &Value) -> Vec<String> {
    validator
        .iter_errors(document)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
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

/// Three guns, 7 m/s wind from 250°, an illumination burst 150 m above a target about 300 m
/// out, charge 1 fired, and gentle terrain under the lead gun's line.
fn illumination_inputs() -> FireMissionInputs {
    FireMissionInputs {
        catalog_id: "test-mortars".to_owned(),
        catalog_version: 3,
        weapon_id: "m252".to_owned(),
        shell_id: "m853-illumination".to_owned(),
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
        crest_profile: Some(profile(&[
            (50.0, 14.0),
            (120.0, 22.0),
            (200.0, 30.0),
            (280.0, 26.0),
        ])),
    }
}

/// Two HE guns in a crosswind onto a target in range of the recommended charges.
fn high_explosive_inputs() -> FireMissionInputs {
    FireMissionInputs {
        catalog_id: "test-mortars".to_owned(),
        catalog_version: 3,
        weapon_id: "m252".to_owned(),
        shell_id: "m821-he".to_owned(),
        charge_rings: None,
        target: FireMissionPoint {
            x: 2_600.0,
            y: 3_400.0,
            height_m: 140.0,
            height_source: HeightSource::Dem,
        },
        guns: vec![
            gun("gun-1", 2_000.0, 2_900.0, 120.0),
            gun("gun-2", 2_050.0, 2_880.0, 118.0),
        ],
        wind: Some(FireMissionWind {
            speed_m_s: 6.0,
            from_deg: 250.0,
        }),
        burst_height_m: None,
        crest_profile: None,
    }
}

fn profile(samples: &[(f64, f64)]) -> TerrainProfile {
    TerrainProfile {
        samples: samples
            .iter()
            .map(|&(downrange_m, height_m)| TerrainSample {
                downrange_m,
                height_m,
            })
            .collect(),
    }
}

fn solve(inputs: &FireMissionInputs) -> FireMissionSolution {
    solve_fire_mission(&catalog(), inputs).expect("the fire mission solves")
}

#[test]
fn assembled_solutions_validate_against_the_fire_mission_solution_definition() {
    let validator = validator("FireMissionSolution");
    let mut out_of_range = high_explosive_inputs();
    out_of_range.target.x = 60_000.0;
    let mut refused_fuze = illumination_inputs();
    refused_fuze.burst_height_m = Some(5_000.0);
    for inputs in [
        illumination_inputs(),
        high_explosive_inputs(),
        out_of_range,
        refused_fuze,
    ] {
        let document = serde_json::to_value(solve(&inputs)).expect("the solution serialises");
        let errors = schema_errors(&validator, &document);
        assert!(errors.is_empty(), "{}: {errors:#?}", inputs.shell_id);
    }
}

#[test]
fn every_optional_part_of_the_solution_has_its_expected_presence() {
    let illumination = solve(&illumination_inputs());
    let fuze = illumination.fuze.expect("a burst height sets the fuze");
    assert!(fuze.time_s.is_some() && fuze.burst_aim.is_some());
    assert!(illumination.crest.is_some() && illumination.dispersion.is_some());

    let high_explosive = solve(&high_explosive_inputs());
    assert!(high_explosive.fuze.is_none() && high_explosive.crest.is_none());
    assert_eq!(high_explosive.dispersion, high_explosive.guns[0].dispersion);
    let document = serde_json::to_value(&high_explosive).expect("serialises");
    assert!(document.get("fuze").is_none() && document.get("crest").is_none());

    let mut out_of_range = high_explosive_inputs();
    out_of_range.target.x = 60_000.0;
    out_of_range.burst_height_m = Some(100.0);
    out_of_range.shell_id = "m853-illumination".to_owned();
    let refused = solve(&out_of_range);
    assert!(refused.guns[0].recommended_rings.is_none());
    assert!(refused.dispersion.is_none() && refused.fuze.is_none());
    assert_eq!(
        serde_json::to_value(&refused).expect("serialises")["dispersion"],
        Value::Null
    );

    let mut refused_fuze = illumination_inputs();
    refused_fuze.burst_height_m = Some(5_000.0);
    let fuze = solve(&refused_fuze)
        .fuze
        .expect("the refused fuze is reported");
    assert!(fuze.refusal.is_some() && fuze.time_s.is_none() && fuze.burst_aim.is_none());
}

#[test]
fn inputs_project_the_fire_mission_save_body() {
    let mut inputs = illumination_inputs();
    inputs.crest_profile = None;
    let mut body = serde_json::to_value(&inputs).expect("the inputs serialise");
    body["event_id"] = Value::Null;
    body["target_grid"] = json!("024 018");
    body["client_solution"] = serde_json::to_value(solve(&inputs)).expect("serialises");
    let errors = schema_errors(&validator("FireMissionSave"), &body);
    assert!(errors.is_empty(), "{errors:#?}");

    let read: FireMissionInputs = serde_json::from_value(body).expect("the save body reads");
    assert_eq!(read, inputs);

    let mut minimal = serde_json::to_value(high_explosive_inputs()).expect("serialises");
    minimal.as_object_mut().expect("object").remove("wind");
    let read: FireMissionInputs = serde_json::from_value(minimal).expect("optionals default");
    assert!(read.wind.is_none() && read.charge_rings.is_none() && read.burst_height_m.is_none());
}

#[test]
fn the_same_inputs_give_identical_bytes() {
    for inputs in [illumination_inputs(), high_explosive_inputs()] {
        let first = serde_json::to_vec(&solve(&inputs)).expect("serialises");
        let decoded_again = solve_fire_mission(&catalog(), &inputs.clone()).expect("solves");
        let second = serde_json::to_vec(&decoded_again).expect("serialises");
        assert_eq!(first, second, "{}", inputs.shell_id);
        let round_trip: FireMissionSolution =
            serde_json::from_slice(&first).expect("the solution reads back");
        assert_eq!(serde_json::to_vec(&round_trip).expect("serialises"), first);
    }
}

#[test]
fn a_three_gun_illumination_battery_in_wind_carries_each_gun_its_fuze_and_crest() {
    let catalog = catalog();
    let inputs = illumination_inputs();
    let solution = solve_fire_mission(&catalog, &inputs).expect("solves");
    assert_eq!(solution.catalog_id, "test-mortars");
    assert_eq!(solution.catalog_version, 3);
    assert_eq!(solution.solver_revision, SOLVER_REVISION);

    let wind = Wind {
        speed_m_s: 7.0,
        from_deg: 250.0,
    };
    let target = MapPosition {
        x_m: 240.0,
        y_m: 180.0,
        height_m: 25.0,
    };
    let battery_guns: Vec<BatteryGun> = inputs
        .guns
        .iter()
        .map(|gun| BatteryGun {
            label: gun.label.clone(),
            position: MapPosition {
                x_m: gun.x,
                y_m: gun.y,
                height_m: gun.height_m,
            },
        })
        .collect();
    let battery = solve_battery(
        &catalog,
        &BatteryRequest {
            weapon_id: "m252",
            shell_id: "m853-illumination",
            guns: &battery_guns,
            target,
            wind,
        },
    )
    .expect("the battery solves");
    assert_eq!(solution.guns, battery);
    assert_eq!(solution.guns.len(), 3);
    for (index, gun) in solution.guns.iter().enumerate() {
        assert_eq!(gun.gun_index as usize, index);
        let row = gun
            .charges
            .iter()
            .find(|row| row.rings == 1)
            .expect("ring 1");
        assert!(row.solves(), "gun {index}: {:?}", row.refusal);
        assert!(row.deflection_correction_mils.expect("solved").abs() > 0.0);
    }

    let lead_request = FireSolutionRequest {
        weapon_id: "m252",
        shell_id: "m853-illumination",
        gun: battery_guns[0].position,
        target,
        wind,
    };
    let fired_row = solution.guns[0]
        .charges
        .iter()
        .find(|row| row.rings == 1)
        .expect("ring 1");
    let dispersion = charge_dispersion(&catalog, &lead_request, fired_row)
        .expect("the fired charge disperses")
        .dispersion;
    assert_eq!(solution.dispersion, Some(dispersion));

    let time_fuze = solve_time_fuze(&catalog, &lead_request, 1, 150.0).expect("the fuze sets");
    let fuze = solution.fuze.expect("the fuze is present");
    assert_eq!(fuze, FireMissionFuze::from_time_fuze(&time_fuze));
    let burst_aim = fuze.burst_aim.expect("the burst aim is laid");
    assert_eq!(burst_aim.rings, 1);
    assert_eq!(
        Some(burst_aim.elevation_mils),
        time_fuze.burst_aim.elevation_mils
    );
    assert_eq!(
        Some(burst_aim.aim_azimuth_mils),
        time_fuze.burst_aim.aim_azimuth_mils
    );
    assert_ne!(Some(burst_aim.elevation_mils), fired_row.elevation_mils);

    let crest = solution.crest.expect("a profile gives a crest clearance");
    assert!(
        !crest.is_blocked() && crest.min_clearance_m > 0.0,
        "{crest:?}"
    );
    let mut ridge = inputs.clone();
    ridge.crest_profile = Some(profile(&[(60.0, 12.0), (150.0, 1_000.0), (240.0, 20.0)]));
    let blocked = solve_fire_mission(&catalog, &ridge)
        .expect("solves")
        .crest
        .expect("crest");
    assert_eq!(blocked.first_blocking_downrange_m, Some(150.0));
}

#[test]
fn the_fired_charge_is_the_operator_charge_or_the_lead_recommendation() {
    let at_distance = |distance_m: f64| {
        let mut inputs = high_explosive_inputs();
        inputs.target.x = inputs.guns[0].x;
        inputs.target.y = inputs.guns[0].y + distance_m;
        inputs
    };
    let (base, recommended, other_rings) = [300.0, 500.0, 800.0, 1_200.0, 1_600.0, 2_000.0]
        .into_iter()
        .find_map(|distance_m| {
            let inputs = at_distance(distance_m);
            let solution = solve(&inputs);
            let lead = &solution.guns[0];
            let lead_rings = lead.recommended_rings?;
            let other = lead
                .charges
                .iter()
                .find(|row| row.rings != lead_rings && row.solves())?
                .rings;
            Some((inputs, solution, other))
        })
        .expect("a fixture distance has two solving charges");
    let mut forced = base.clone();
    forced.charge_rings = Some(other_rings);
    let forced_solution = solve(&forced);
    assert_eq!(forced_solution.guns, recommended.guns);
    assert_eq!(recommended.dispersion, recommended.guns[0].dispersion);
    assert!(forced_solution.dispersion.is_some());
    assert_ne!(forced_solution.dispersion, recommended.dispersion);
}

#[test]
fn malformed_missions_are_refused() {
    let catalog = catalog();
    let mut wrong_version = high_explosive_inputs();
    wrong_version.catalog_version = 4;
    assert!(matches!(
        solve_fire_mission(&catalog, &wrong_version),
        Err(FireMissionRefusal::CatalogMismatch {
            requested_version: 4,
            held_version: 3,
            ..
        })
    ));
    let mut wrong_id = high_explosive_inputs();
    wrong_id.catalog_id = "other".to_owned();
    assert!(matches!(
        solve_fire_mission(&catalog, &wrong_id),
        Err(FireMissionRefusal::CatalogMismatch { .. })
    ));
    let mut unknown_charge = high_explosive_inputs();
    unknown_charge.charge_rings = Some(7);
    assert!(matches!(
        solve_fire_mission(&catalog, &unknown_charge),
        Err(FireMissionRefusal::UnknownCharge(
            CatalogLookupError::UnknownRing { rings: 7, .. }
        ))
    ));
    let mut no_fuze = high_explosive_inputs();
    no_fuze.burst_height_m = Some(100.0);
    assert!(matches!(
        solve_fire_mission(&catalog, &no_fuze),
        Err(FireMissionRefusal::Fuze(
            FuzeError::ShellHasNoTimeFuze { .. }
        ))
    ));
    let mut no_guns = high_explosive_inputs();
    no_guns.guns.clear();
    assert!(matches!(
        solve_fire_mission(&catalog, &no_guns),
        Err(FireMissionRefusal::Battery(BatteryError::NoGuns))
    ));
    let mut beyond_impact = illumination_inputs();
    beyond_impact.crest_profile = Some(profile(&[(5_000.0, 0.0)]));
    assert!(matches!(
        solve_fire_mission(&catalog, &beyond_impact),
        Err(FireMissionRefusal::Crest(
            CrestClearanceError::NoSampleUnderFlight
        ))
    ));
    let mut bad_wind = high_explosive_inputs();
    bad_wind.wind = Some(FireMissionWind {
        speed_m_s: f64::NAN,
        from_deg: 0.0,
    });
    assert!(matches!(
        solve_fire_mission(&catalog, &bad_wind),
        Err(FireMissionRefusal::Battery(BatteryError::Gun {
            gun_index: 0,
            ..
        }))
    ));
}

/// The committed vanilla catalog (`vanilla_mortars` v1: `m252` fires the time-fuzed `m853a1`
/// at rings 1 to 4, fuze window 10–40 s); a missing file fails the build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));

fn vanilla_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON).expect("the vanilla catalog decodes")
}

/// The mortar walkthrough: one `m252` firing `m853a1` 1,200 m north onto a target 23.5 m
/// above the gun, calm air, no operator charge, a burst `burst_height_m` above the target.
fn walkthrough_inputs(burst_height_m: f64) -> FireMissionInputs {
    FireMissionInputs {
        catalog_id: "vanilla_mortars".to_owned(),
        catalog_version: 1,
        weapon_id: "m252".to_owned(),
        shell_id: "m853a1".to_owned(),
        charge_rings: None,
        target: FireMissionPoint {
            x: 0.0,
            y: 1_200.0,
            height_m: 123.5,
            height_source: HeightSource::Manual,
        },
        guns: vec![gun("gun-1", 0.0, 0.0, 100.0)],
        wind: None,
        burst_height_m: Some(burst_height_m),
        crest_profile: None,
    }
}

fn walkthrough_lead_request() -> FireSolutionRequest<'static> {
    FireSolutionRequest {
        weapon_id: "m252",
        shell_id: "m853a1",
        gun: MapPosition {
            x_m: 0.0,
            y_m: 0.0,
            height_m: 100.0,
        },
        target: MapPosition {
            x_m: 0.0,
            y_m: 1_200.0,
            height_m: 123.5,
        },
        wind: Wind::CALM,
    }
}

#[test]
fn a_high_burst_is_fuzed_at_the_lowest_charge_that_reaches_it_in_the_window() {
    let catalog = vanilla_catalog();
    let solution = solve_fire_mission(&catalog, &walkthrough_inputs(300.0)).expect("solves");
    let ground_rings = solution.guns[0]
        .recommended_rings
        .expect("the ground target solves");
    let fuze = solution.fuze.expect("a burst height sets the fuze");
    assert_eq!(fuze.refusal, None, "{fuze:?}");
    let time_s = fuze.time_s.expect("the burst is fuzed");
    assert!((10.0..=40.0).contains(&time_s), "{time_s} s");
    let aim = fuze.burst_aim.expect("the burst aim is laid");
    assert!(aim.rings > ground_rings, "{} vs {ground_rings}", aim.rings);

    let request = walkthrough_lead_request();
    for rings in [1, 2, 3, 4].into_iter().filter(|rings| *rings < aim.rings) {
        let lower = solve_time_fuze(&catalog, &request, rings, 300.0).expect("computes");
        assert_eq!(lower.setting.time_s, None, "charge {rings} would fuze");
    }
    let at_burst_rings = solve_time_fuze(&catalog, &request, aim.rings, 300.0).expect("computes");
    assert_eq!(fuze, FireMissionFuze::from_time_fuze(&at_burst_rings));
    assert_eq!(
        solution.dispersion,
        fired_charge_dispersion(&catalog, &request, &solution.guns[0], aim.rings)
            .expect("disperses")
    );
}

#[test]
fn a_low_burst_keeps_the_ground_charge() {
    let catalog = vanilla_catalog();
    let solution = solve_fire_mission(&catalog, &walkthrough_inputs(100.0)).expect("solves");
    let lead = &solution.guns[0];
    let ground_rings = lead.recommended_rings.expect("the ground target solves");
    let fuze = solution.fuze.expect("a burst height sets the fuze");
    let expected = solve_time_fuze(&catalog, &walkthrough_lead_request(), ground_rings, 100.0)
        .expect("computes");
    assert_eq!(fuze, FireMissionFuze::from_time_fuze(&expected));
    assert_eq!(fuze.burst_aim.expect("laid").rings, ground_rings);
    assert!(
        (10.0..=40.0).contains(&fuze.time_s.expect("fuzed")),
        "{fuze:?}"
    );
    assert_eq!(solution.dispersion, lead.dispersion);
}

#[test]
fn a_burst_no_charge_can_fuze_names_the_cause() {
    let catalog = vanilla_catalog();
    let unreachable = solve_fire_mission(&catalog, &walkthrough_inputs(5_000.0)).expect("solves");
    let fuze = unreachable.fuze.expect("the refused fuze is reported");
    assert_eq!(fuze.refusal, Some(FuzeRefusal::AboveApex));
    assert!(fuze.time_s.is_none() && fuze.burst_aim.is_none());
    assert_eq!(unreachable.dispersion, unreachable.guns[0].dispersion);

    let mut narrow_window = vanilla_catalog();
    narrow_window
        .shells
        .iter_mut()
        .find(|shell| shell.shell_id == "m853a1")
        .expect("the shell exists")
        .time_fuze = Some(crate::data::scenario::ballistics::catalog::TimeFuze {
        min_s: 0.0,
        max_s: 1.0,
        default_s: 0.5,
    });
    let outside = solve_fire_mission(&narrow_window, &walkthrough_inputs(300.0))
        .expect("solves")
        .fuze
        .expect("the refused fuze is reported");
    assert_eq!(outside.refusal, Some(FuzeRefusal::OutsideFuzeWindow));
    assert!(outside.time_s.is_none() && outside.burst_aim.is_none());
}
