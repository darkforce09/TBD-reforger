//! Captured-response round trips for the fire missions: an event's saved list, the save answer,
//! and the solution, save-body and stored-mission shapes of the catalog model.

use super::*;

/// The saved fire missions of one event. The capture holds a row stored before the charge,
/// azimuth in mils and flight time were recorded (all three `null`) and one that records them, so
/// both arms of the nullable figures round-trip as sent. Both rows predate catalogs, so every
/// catalog-model field is `null` and neither row has a gun.
#[test]
fn saved_fire_missions_of_an_event() {
    const G: &str =
        golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__fire-missions.json");
    assert_golden::<DataEnvelope<SavedFire>>(G, &[]);
    let list: DataEnvelope<SavedFire> = serde_json::from_str(G).unwrap();
    assert!(list
        .data
        .iter()
        .all(|row| row.event_id.as_deref() == Some("c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7")));
    assert!(list.data.iter().any(|row| row.charge.is_none()
        && row.azimuth_mils.is_none()
        && row.time_of_flight_s.is_none()));
    assert!(list.data.iter().any(|row| row.charge == Some(1)
        && row.azimuth_mils == Some(0)
        && row.time_of_flight_s == Some(18.3)));
    assert!(list.data.iter().all(|row| row.catalog_id.is_none()
        && row.catalog_version.is_none()
        && row.solver_revision.is_none()
        && row.guns.is_empty()));
}

/// A fire mission saved with no event carries no `event_id` key, and it round-trips that way.
#[test]
fn a_fire_mission_saved_without_an_event_has_no_event_id_key() {
    let list: Value = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__fire-missions.json"
    ))
    .unwrap();
    let mut row = list["data"][1].clone();
    row.as_object_mut().unwrap().remove("event_id");
    let wire = row.to_string();
    assert_golden::<SavedFire>(&wire, &[]);
    let decoded: SavedFire = serde_json::from_str(&wire).unwrap();
    assert!(decoded.event_id.is_none());
}

/// The captured answer to saving a two-gun battery in a crosswind, decoded with the type the
/// mortar page decodes it with: the stored mission names the catalog version, solver, wind and
/// guns of the solution, and each stored gun lays its fired charge — the recommended one, since
/// the save names no charge — at that charge's wind-corrected aim azimuth.
#[test]
fn fire_mission_saved() {
    const G: &str = golden!("POST__fire-missions.json");
    assert_golden::<SavedFireMissionAnswer>(G, &[]);
    let saved: SavedFireMissionAnswer = serde_json::from_str(G).unwrap();
    let (solution, stored) = (&saved.solution, &saved.fire_mission);
    assert_eq!(
        stored.catalog_id.as_deref(),
        Some(solution.catalog_id.as_str())
    );
    assert_eq!(stored.catalog_version, Some(solution.catalog_version));
    assert_eq!(
        stored.solver_revision.as_deref(),
        Some(solution.solver_revision.as_str())
    );
    assert_eq!(stored.charge_rings, None);
    assert_eq!(stored.wind_speed_m_s, Some(3.0));
    assert_eq!(stored.wind_from_deg, Some(270.0));
    assert_eq!(stored.guns.len(), 2);
    assert_eq!(stored.guns.len(), solution.guns.len());
    for (stored_gun, solved_gun) in stored.guns.iter().zip(&solution.guns) {
        assert_eq!(stored_gun.gun_index, solved_gun.gun_index);
        assert_eq!(stored_gun.label, solved_gun.label);
        let fired = solved_gun
            .charges
            .iter()
            .find(|row| Some(row.rings) == solved_gun.recommended_rings)
            .expect("each gun of the capture has a recommended charge");
        assert_eq!(stored_gun.charge_rings, solved_gun.recommended_rings);
        assert_eq!(Some(stored_gun.azimuth_mils), fired.aim_azimuth_mils);
        assert_ne!(stored_gun.azimuth_mils, solved_gun.azimuth_mils);
        assert_eq!(stored_gun.elevation_mils, fired.elevation_mils);
        assert_eq!(stored_gun.time_of_flight_s, fired.time_of_flight_s);
    }
}

// ── catalog-model shapes no capture carries in every arm ──

fn dispersion() -> Value {
    json!({
        "range_probable_error_m": 14.5,
        "deflection_probable_error_m": 6.25,
        "ellipse_semi_major_m": 25.5,
        "ellipse_semi_minor_m": 11.0,
        "ellipse_orientation_deg": 99.5,
        "standard_dispersion_m": 30.0,
        "verified_in_engine": false
    })
}

/// A two-gun battery: the lead gun solves on its second charge, the second gun solves nothing.
fn battery_solution() -> Value {
    let lead_gun = json!({
        "gun_index": 0, "label": "Gun 1", "distance_m": 1217.5,
        "height_difference_m": -12.25, "azimuth_deg": 99.5, "azimuth_mils": 1768.5,
        "mils_per_circle": 6400,
        "charges": [
            {"rings": 0, "elevation_deg": null, "elevation_mils": null,
             "time_of_flight_s": null, "apex_m": null, "aim_azimuth_deg": null,
             "aim_azimuth_mils": null, "deflection_correction_mils": null,
             "range_correction_m": null, "refusal": "out_of_range"},
            {"rings": 1, "elevation_deg": 74.5, "elevation_mils": 1324.5,
             "time_of_flight_s": 29.5, "apex_m": 812.5, "aim_azimuth_deg": 100.25,
             "aim_azimuth_mils": 1782.5, "deflection_correction_mils": 14.0,
             "range_correction_m": -6.5, "refusal": null}
        ],
        "recommended_rings": 1,
        "dispersion": dispersion()
    });
    let unsolved_gun = json!({
        "gun_index": 1, "label": "Gun 2", "distance_m": 40.5,
        "height_difference_m": 0.5, "azimuth_deg": 12.5, "azimuth_mils": 222.5,
        "mils_per_circle": 6400,
        "charges": [
            {"rings": 0, "elevation_deg": null, "elevation_mils": null,
             "time_of_flight_s": null, "apex_m": null, "aim_azimuth_deg": null,
             "aim_azimuth_mils": null, "deflection_correction_mils": null,
             "range_correction_m": null, "refusal": "too_close"}
        ],
        "recommended_rings": null,
        "dispersion": null
    });
    json!({
        "guns": [lead_gun, unsolved_gun],
        "dispersion": dispersion(),
        "fuze": {"burst_height_m": 250.5, "time_s": null, "min_s": 1.5, "max_s": 25.5,
                 "default_s": 10.5, "refusal": "outside_fuze_window", "burst_aim": null},
        "crest": {"min_clearance_m": -3.5, "min_clearance_downrange_m": 410.5,
                  "first_blocking_downrange_m": 400.5},
        "catalog_id": "vanilla-mortars",
        "catalog_version": 1,
        "solver_revision": "rk4-dt0.01-v1"
    })
}

/// [`battery_solution`] with its fuze set: a fuze time and the burst-point aim that produces it.
fn set_fuze_solution() -> Value {
    let mut solution = battery_solution();
    solution["fuze"] = json!({
        "burst_height_m": 250.5, "time_s": 27.5, "min_s": 1.5, "max_s": 45.5,
        "default_s": 10.5, "refusal": null,
        "burst_aim": {"rings": 2, "aim_azimuth_deg": 100.25, "aim_azimuth_mils": 1782.5,
                      "elevation_deg": 71.5, "elevation_mils": 1271.25}
    });
    solution
}

/// A mission stored with a catalog: its legacy figures, every catalog-model field, and a gun
/// with and without a solution.
fn catalog_model_mission() -> Value {
    let mut mission = json!({
        "id": "99999999-9999-4999-9999-999999999999",
        "event_id": "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7",
        "created_by": "000000000000000001",
        "weapon_system": "M252 81mm", "fp_grid": "1000, 2000", "target_grid": "2200, 1800",
        "distance_m": 1217, "azimuth_deg": 99.5, "elevation_mils": 1324,
        "fp_x": 1000.5, "fp_y": 2000.5, "tgt_x": 2200.5, "tgt_y": 1800.5,
        "azimuth_mils": 1768, "charge": 1, "time_of_flight_s": 29.5,
        "created_at": "2000-01-01T00:00:00Z"
    });
    let catalog_model = json!({
        "catalog_id": "vanilla-mortars", "catalog_version": 1,
        "weapon_id": "m252", "shell_id": "m821-he", "charge_rings": null,
        "target_height_m": 88.5, "target_height_source": "dem",
        "wind_speed_m_s": 4.5, "wind_from_deg": 270.5, "burst_height_m": null,
        "fuze_time_s": null, "mils_per_circle": 6400, "dispersion": dispersion(),
        "solver_revision": "rk4-dt0.01-v1"
    });
    let guns = json!([
        {"gun_index": 0, "label": "Gun 1", "x": 1000.5, "y": 2000.5, "height_m": 100.75,
         "height_source": "dem", "azimuth_mils": 1768.5, "elevation_mils": 1324.5,
         "charge_rings": 1, "time_of_flight_s": 29.5},
        {"gun_index": 1, "label": "Gun 2", "x": 2190.5, "y": 1760.5, "height_m": 88.0,
         "height_source": "manual", "azimuth_mils": 222.5, "elevation_mils": null,
         "charge_rings": null, "time_of_flight_s": null}
    ]);
    let object = mission.as_object_mut().unwrap();
    object.extend(catalog_model.as_object().unwrap().clone());
    object.insert("guns".to_string(), guns);
    mission
}

/// Every key of a catalog-model save answer — the per-charge refusals, both dispersion arms, the
/// fuze and crest, and a stored gun with and without a solution — is a named field, and the
/// nulls stay explicit.
#[test]
fn a_catalog_model_save_answer_claims_every_key() {
    let wire = json!({"solution": battery_solution(), "fire_mission": catalog_model_mission()})
        .to_string();
    assert_golden::<SavedFireMissionAnswer>(&wire, &[]);
    let saved: SavedFireMissionAnswer = serde_json::from_str(&wire).unwrap();
    let lead = &saved.solution.guns[0];
    assert_eq!(lead.recommended_rings, Some(1));
    assert_eq!(lead.charges[0].refusal, Some(SolutionRefusal::OutOfRange));
    assert_eq!(lead.charges[1].time_of_flight_s, Some(29.5));
    assert_eq!(saved.solution.guns[1].recommended_rings, None);
    assert_eq!(
        saved.solution.fuze.as_ref().and_then(|fuze| fuze.refusal),
        Some(FuzeRefusal::OutsideFuzeWindow)
    );
    assert_eq!(
        saved.fire_mission.target_height_source,
        Some(HeightSource::Dem)
    );
    assert_eq!(
        saved.fire_mission.guns[1].height_source,
        HeightSource::Manual
    );
    assert_eq!(saved.fire_mission.guns[1].elevation_mils, None);
}

/// A solution with no fuze and no crest carries neither key, and writes neither back as `null`.
#[test]
fn a_solution_without_fuze_or_crest_carries_neither_key() {
    let mut solution = battery_solution();
    let object = solution.as_object_mut().unwrap();
    object.remove("fuze");
    object.remove("crest");
    let wire = solution.to_string();
    assert_golden::<FireMissionSolution>(&wire, &[]);
    let decoded: FireMissionSolution = serde_json::from_str(&wire).unwrap();
    assert!(decoded.fuze.is_none() && decoded.crest.is_none());
}

/// The save body round-trips with every optional input and with none of them: an absent
/// `event_id`, `charge_rings`, `wind` or `burst_height_m` stays absent.
#[test]
fn a_fire_mission_save_body_round_trips_with_and_without_its_optional_inputs() {
    let full = json!({
        "event_id": "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7",
        "catalog_id": "vanilla-mortars", "catalog_version": 1,
        "weapon_id": "m252", "shell_id": "m853-illumination", "charge_rings": 2,
        "target": {"x": 2200.5, "y": 1800.5, "height_m": 88.5, "height_source": "dem"},
        "guns": [{"label": "Gun 1", "x": 1000.5, "y": 2000.5, "height_m": 100.75,
                  "height_source": "manual"}],
        "wind": {"speed_m_s": 4.5, "from_deg": 270.5},
        "burst_height_m": 250.5,
        "target_grid": "022018",
        "client_solution": set_fuze_solution()
    });
    assert_golden::<FireMissionSave>(&full.to_string(), &[]);
    let decoded: FireMissionSave = serde_json::from_value(full.clone()).unwrap();
    let fuze = decoded.client_solution.fuze.expect("the set fuze decodes");
    assert_eq!(fuze.time_s, Some(27.5));
    assert_eq!(fuze.burst_aim.map(|aim| aim.rings), Some(2));

    let mut minimal = full.clone();
    let object = minimal.as_object_mut().unwrap();
    for key in ["event_id", "charge_rings", "wind", "burst_height_m"] {
        object.remove(key);
    }
    let wire = minimal.to_string();
    assert_golden::<FireMissionSave>(&wire, &[]);
    let decoded: FireMissionSave = serde_json::from_str(&wire).unwrap();
    assert!(decoded.event_id.is_none() && decoded.charge_rings.is_none());
    assert!(decoded.wind.is_none() && decoded.burst_height_m.is_none());
}

/// The dispersion is a documented interpretation: a stored mission whose dispersion claims an
/// in-engine verification fails the read, and one that disclaims it decodes. The refusal is the
/// engine dispersion type's own deserializer, reached through the stored mission.
#[test]
fn a_dispersion_claiming_in_engine_verification_fails_the_read() {
    let mut claimed = catalog_model_mission();
    claimed["dispersion"]["verified_in_engine"] = json!(true);
    assert!(serde_json::from_value::<SavedFire>(claimed).is_err());
    let decoded: SavedFire = serde_json::from_value(catalog_model_mission()).unwrap();
    assert_eq!(
        decoded.dispersion.map(|spread| spread.verified_in_engine),
        Some(false)
    );
}

/// The refusal sets are closed: a charge or fuze refusal outside the contract's values fails
/// the read.
#[test]
fn an_unknown_refusal_fails_the_read() {
    let mut solution = battery_solution();
    solution["guns"][0]["charges"][0]["refusal"] = json!("jammed");
    assert!(serde_json::from_value::<FireMissionSolution>(solution).is_err());

    let mut solution = battery_solution();
    solution["fuze"]["refusal"] = json!("dud");
    assert!(serde_json::from_value::<FireMissionSolution>(solution).is_err());
}

/// Every fuze refusal the contract names reads and writes back as the same string, and the
/// contract's set is exactly the one the DTO reads.
#[test]
fn every_contract_fuze_refusal_round_trips() {
    const SCHEMA: &str = include_str!(
        "../../../../../../../../../contracts_v2/definitions/fire-mission.schema.json"
    );
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let values = schema["definitions"]["FuzeRefusal"]["enum"]
        .as_array()
        .expect("the contract lists the fuze refusals");
    assert_eq!(values.len(), 7, "{values:?}");
    for value in values {
        let mut solution = battery_solution();
        solution["fuze"]["refusal"] = value.clone();
        let read: FireMissionSolution = serde_json::from_value(solution).unwrap();
        let refusal = read.fuze.and_then(|fuze| fuze.refusal).expect("refused");
        assert_eq!(&serde_json::to_value(refusal).unwrap(), value);
    }
    let mut solution = battery_solution();
    solution["fuze"]["refusal"] = json!("no_burst_crossing");
    assert!(serde_json::from_value::<FireMissionSolution>(solution).is_err());
}
