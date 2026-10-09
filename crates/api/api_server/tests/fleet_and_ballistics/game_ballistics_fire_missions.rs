//! The fire-mission routes through the real HTTP router: `POST /api/v1/fire-missions`
//! re-solves a save against its pinned ballistics catalog and stores the server's solution with
//! one row per gun, refuses a client solution that disagrees with it, and
//! `GET /api/v1/events/{id}/fire-missions` lists missions to the viewers the event's access rules
//! admit.
//!
//! Every case needs `TEST_DATABASE_URL`; without it the suite fails with that cause. The
//! committed vanilla catalog is stored straight into `ballistics_catalogs` (the upload route and
//! its calibration have their own module, `game_ballistics_catalog_upload`), and each
//! client solution is computed here with the same `fire_mission_planning` assembler the calculator runs.
//! Numbers are asserted on the database rows read back with direct queries, not only on the
//! answer, and answers are checked against `fire-mission.schema.json`.

use crate::{content_support, contract_support};

use axum::http::StatusCode;
use ballistics_model::catalog::BallisticsCatalog;
use fire_mission_planning::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionSolution, HeightSource,
    SOLVER_REVISION, solve_fire_mission,
};
use fire_mission_planning::fire_mission_comparison::compare_solutions;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use content_support::{Actor, ContentSuite};
use contract_support::assert_valid;

const SUITE: &str = "game_ballistics_fire_missions";
const CONTRACT: &str = "fire-mission.schema.json";
const SAVE_URI: &str = "/api/v1/fire-missions";
/// SHA-256 of the committed catalog bytes.
const VANILLA_CATALOG_SHA256: &str =
    "24a68cc5e22b5d3dc62ec80d82d0e004916e1d300ac0a964d1660847e8f9fbde";
/// SHA-256 of the committed calibration bundle.
const VANILLA_CALIBRATION_SHA256: &str =
    "12be201bbdd22f1bfa72b3aead176cc53969abeca532e33ee8e7216b2fc2d3d0";
/// Tolerance for a number that crossed the JSON wire against its in-process twin.
const WIRE_EPSILON: f64 = 1e-9;

fn vanilla_catalog_bytes() -> Vec<u8> {
    let path = repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the repository root above the API package")
    .join("contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json");
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read committed catalog {}: {error}", path.display()))
}

fn vanilla_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(&vanilla_catalog_bytes()).expect("decode vanilla catalog")
}

/// A case's router with the vanilla catalog version stored (once per database).
async fn fixture() -> ContentSuite {
    let suite = ContentSuite::new(SUITE).await;
    let text = String::from_utf8(vanilla_catalog_bytes()).expect("the bytes are UTF-8");
    let document: Value = serde_json::from_str(&text).expect("the text decodes as JSON");
    sqlx::query(
        "INSERT INTO ballistics_catalogs (catalog_id, catalog_version, title, game_build, \
         export_generation_id, catalog_sha256, calibration_sha256, catalog_document, \
         calibration_document, validation_report, uploaded_by) \
         VALUES ('vanilla_mortars', 1, $1, $2, $3, $4, $5, $6::jsonb, '{}'::jsonb, '{}'::jsonb, $7) \
         ON CONFLICT DO NOTHING",
    )
    .bind(document["title"].as_str().expect("the `title` field is a string"))
    .bind(document["game_build"].as_str().expect("the `game_build` field is a string"))
    .bind(document["export_generation_id"].as_str().expect("the `export_generation_id` field is a string"))
    .bind(VANILLA_CATALOG_SHA256)
    .bind(VANILLA_CALIBRATION_SHA256)
    .bind(&text)
    .bind(&suite.admin.id)
    .execute(suite.pool())
    .await
    .expect("store the vanilla catalog version");
    suite
}

/// An event with the default members-only access policy.
async fn plant_event(suite: &ContentSuite) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, created_by, created_at) \
         VALUES ('Fire mission fixture', now() + interval '2 days', $1, now()) RETURNING id",
    )
    .bind(&suite.admin.id)
    .fetch_one(suite.pool())
    .await
    .expect("plant an event")
}

fn gun(label: &str, x: f64, y: f64, height_m: f64) -> FireMissionGunPosition {
    FireMissionGunPosition {
        label: label.to_owned(),
        x,
        y,
        height_m,
        height_source: HeightSource::Manual,
    }
}

/// M252 firing M821 HE from `guns` onto (2200, 1800, 40 m DEM) in calm air.
fn he_inputs(guns: Vec<FireMissionGunPosition>) -> FireMissionInputs {
    FireMissionInputs {
        catalog_id: "vanilla_mortars".into(),
        catalog_version: 1,
        weapon_id: "m252".into(),
        shell_id: "m821".into(),
        charge_rings: None,
        target: FireMissionPoint {
            x: 2200.0,
            y: 1800.0,
            height_m: 40.0,
            height_source: HeightSource::Dem,
        },
        guns,
        wind: None,
        burst_height_m: None,
        crest_profile: None,
    }
}

fn solve(inputs: &FireMissionInputs) -> FireMissionSolution {
    solve_fire_mission(&vanilla_catalog(), inputs).expect("the fixture inputs solve")
}

/// A `FireMissionSave` body of `inputs`, checked against the contract.
fn save_body(
    inputs: &FireMissionInputs,
    event: Option<Uuid>,
    client: &FireMissionSolution,
) -> Value {
    let mut body = serde_json::to_value(inputs).expect("the value serialises to JSON");
    body["target_grid"] = json!("0220 0180");
    body["client_solution"] = serde_json::to_value(client).expect("the value serialises to JSON");
    if let Some(event) = event {
        body["event_id"] = json!(event.to_string());
    }
    assert_valid(CONTRACT, Some("FireMissionSave"), &body);
    body
}

async fn save(suite: &ContentSuite, actor: &Actor, body: Value) -> (StatusCode, Value) {
    suite.call(Some(actor), "POST", SAVE_URI, Some(body)).await
}

async fn list(suite: &ContentSuite, actor: &Actor, event: Uuid) -> (StatusCode, Value) {
    let uri = format!("/api/v1/events/{event}/fire-missions");
    suite.call(Some(actor), "GET", &uri, None).await
}

/// One stored gun row, straight out of `fire_mission_guns`.
type GunRow = (
    i16,
    String,
    f64,
    f64,
    f64,
    String,
    f64,
    Option<f64>,
    Option<i16>,
    Option<f64>,
);

/// Catalog provenance of a stored mission: catalog id and version, weapon, shell, mils per
/// circle and solver revision.
type StoredProvenance = (
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
);

/// Operator charge, target height and source, target and lead-gun eastings, the lead gun's
/// fired charge and `fp_grid` of a stored mission.
type StoredPositions = (
    Option<i16>,
    Option<f64>,
    Option<String>,
    Option<f64>,
    Option<f64>,
    Option<i64>,
    String,
);

async fn gun_rows(pool: &PgPool, mission: &str) -> Vec<GunRow> {
    sqlx::query_as(
        "SELECT gun_index, label, x, y, height_m, height_source, azimuth_mils, elevation_mils, \
         charge_rings, time_of_flight_s FROM fire_mission_guns \
         WHERE fire_mission_id = $1::uuid ORDER BY gun_index",
    )
    .bind(mission)
    .fetch_all(pool)
    .await
    .expect("read the stored guns")
}

async fn missions_by(pool: &PgPool, actor: &Actor) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM fire_missions WHERE created_by = $1")
        .bind(&actor.id)
        .fetch_one(pool)
        .await
        .expect("the read of fire_missions returns a row")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= WIRE_EPSILON
}

/// Asserts every stored gun row carries the server solution's fired (recommended) charge.
fn assert_guns_match(rows: &[GunRow], solution: &FireMissionSolution, inputs: &FireMissionInputs) {
    assert_eq!(rows.len(), solution.guns.len(), "one stored row per gun");
    for (row, (gun, input)) in rows.iter().zip(solution.guns.iter().zip(&inputs.guns)) {
        let rings = gun.recommended_rings.expect("the fixture guns solve");
        let fired = gun
            .charges
            .iter()
            .find(|c| c.rings == rings)
            .expect("the gun has a charge with the requested ring count");
        assert_eq!(i64::from(row.0), i64::from(gun.gun_index));
        assert_eq!(
            (row.1.as_str(), row.2, row.3, row.4),
            (input.label.as_str(), input.x, input.y, input.height_m)
        );
        assert_eq!(row.5, "manual");
        assert_eq!(
            row.6,
            fired
                .aim_azimuth_mils
                .expect("the fired solution carries an aim azimuth")
        );
        assert_eq!(row.7, fired.elevation_mils);
        assert_eq!(row.8.map(u32::try_from).map(Result::unwrap), Some(rings));
        assert_eq!(row.9, fired.time_of_flight_s);
    }
}

#[tokio::test]
async fn game_ballistics_fire_mission_agreeing_save_stores_the_server_solution_and_its_gun() {
    let suite = fixture().await;
    let event = plant_event(&suite).await;
    let inputs = he_inputs(vec![gun("Gun 1", 1000.0, 2000.0, 50.0)]);
    let client = solve(&inputs);
    let (status, body) = save(
        &suite,
        &suite.member,
        save_body(&inputs, Some(event), &client),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_valid(CONTRACT, Some("SavedFireMission"), &body);

    let answered: FireMissionSolution = serde_json::from_value(body["solution"].clone()).unwrap();
    assert!(compare_solutions(&client, &answered, 6400).agrees());
    assert!(!SOLVER_REVISION.is_empty());
    assert_eq!(answered.solver_revision, SOLVER_REVISION);
    let mission = &body["fire_mission"];
    assert_eq!(mission["event_id"], event.to_string());
    assert_eq!(mission["created_by"], suite.member.id);
    assert_eq!(mission["catalog_id"], "vanilla_mortars");
    assert_eq!(mission["catalog_version"], 1);
    assert_eq!(mission["weapon_system"], "M252 - Mortar");
    assert_eq!(mission["target_grid"], "0220 0180");
    assert_eq!(mission["guns"].as_array().unwrap().len(), 1);

    let id = mission["id"].as_str().unwrap();
    let rows = gun_rows(suite.pool(), id).await;
    assert_guns_match(&rows, &client, &inputs);
    let provenance: StoredProvenance = sqlx::query_as(
        "SELECT catalog_id, catalog_version, weapon_id, shell_id, mils_per_circle, \
         solver_revision FROM fire_missions WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert_eq!(
        provenance,
        (
            Some("vanilla_mortars".into()),
            Some(1),
            Some("m252".into()),
            Some("m821".into()),
            Some(6400),
            Some(SOLVER_REVISION.to_owned()),
        )
    );
    let lead_rings = client.guns[0].recommended_rings.unwrap();
    let positions: StoredPositions = sqlx::query_as(
        "SELECT charge_rings, target_height_m, target_height_source, tgt_x, fp_x, charge, \
         fp_grid FROM fire_missions WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert_eq!(
        positions,
        (
            None,
            Some(40.0),
            Some("dem".into()),
            Some(2200.0),
            Some(1000.0),
            Some(i64::from(lead_rings)),
            "1000, 2000".into(),
        )
    );
    let answered_elevation = mission["guns"][0]["elevation_mils"].as_f64().unwrap();
    assert!(close(answered_elevation, rows[0].7.unwrap()));

    let (status, listed) = list(&suite, &suite.member, event).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_valid(CONTRACT, Some("FireMissionList"), &listed);
    let row = &listed["data"][0];
    assert_eq!(row["id"], id);
    assert_eq!(row["guns"][0]["label"], "Gun 1");
    assert_eq!(row["guns"][0]["charge_rings"], i64::from(lead_rings));
    suite.pool().close().await;
}

#[tokio::test]
async fn game_ballistics_fire_mission_skewed_client_solution_is_refused_with_both_values() {
    let suite = fixture().await;
    let inputs = he_inputs(vec![gun("Gun 1", 1000.0, 2000.0, 50.0)]);
    let server = solve(&inputs);
    let rings = server.guns[0].recommended_rings.unwrap();
    let row = server.guns[0]
        .charges
        .iter()
        .position(|c| c.rings == rings)
        .unwrap();

    let mut skewed = server.clone();
    let elevation = skewed.guns[0].charges[row].elevation_mils.as_mut().unwrap();
    *elevation += 1.5;
    let (status, body) = save(&suite, &suite.member, save_body(&inputs, None, &skewed)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["details"]["code"], "solution_mismatch");
    let mismatch = &body["details"]["mismatches"][0];
    assert_eq!(mismatch["kind"], "value", "{body}");
    assert_eq!(mismatch["quantity"], "elevation_mils");
    let server_elevation = server.guns[0].charges[row].elevation_mils.unwrap();
    assert!(close(
        mismatch["client"].as_f64().unwrap(),
        server_elevation + 1.5
    ));
    assert!(close(
        mismatch["server"].as_f64().unwrap(),
        server_elevation
    ));
    assert_eq!(mismatch["tolerance"], 1.0);
    assert_eq!(
        missions_by(suite.pool(), &suite.member).await,
        0,
        "a refused save stores nothing"
    );

    // A skew inside the tolerance is accepted, and the stored numbers are the server's.
    let mut near = server.clone();
    *near.guns[0].charges[row].elevation_mils.as_mut().unwrap() += 0.9;
    *near.guns[0].charges[row].time_of_flight_s.as_mut().unwrap() += 0.09;
    let (status, body) = save(&suite, &suite.member, save_body(&inputs, None, &near)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let rows = gun_rows(suite.pool(), body["fire_mission"]["id"].as_str().unwrap()).await;
    assert_eq!(
        rows[0].7,
        Some(server_elevation),
        "the server's elevation is stored"
    );
    assert_eq!(rows[0].9, server.guns[0].charges[row].time_of_flight_s);
    suite.pool().close().await;
}

#[tokio::test]
async fn game_ballistics_fire_mission_list_and_save_follow_viewer_event_access() {
    let suite = fixture().await;
    let event = plant_event(&suite).await;
    let guest = suite.account("guest", "guest").await;
    let inputs = he_inputs(vec![gun("Gun 1", 1000.0, 2000.0, 50.0)]);
    let client = solve(&inputs);
    let (status, _) = save(
        &suite,
        &suite.member,
        save_body(&inputs, Some(event), &client),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let hidden = list(&suite, &guest, event).await;
    let missing = list(&suite, &guest, Uuid::new_v4()).await;
    assert_eq!(hidden.0, StatusCode::NOT_FOUND, "{}", hidden.1);
    assert_eq!(
        hidden, missing,
        "a hidden event lists exactly like a missing one"
    );
    let (status, answer) = save(&suite, &guest, save_body(&inputs, Some(event), &client)).await;
    assert_eq!(
        (status, answer["error"].as_str()),
        (StatusCode::NOT_FOUND, Some("event not found"))
    );
    assert_eq!(missions_by(suite.pool(), &guest).await, 0);

    for viewer in [&suite.member, &suite.admin] {
        let (status, listed) = list(&suite, viewer, event).await;
        assert_eq!(status, StatusCode::OK, "{listed}");
        assert_eq!(listed["data"].as_array().unwrap().len(), 1);
    }
    let anonymous = suite
        .call(
            None,
            "GET",
            &format!("/api/v1/events/{event}/fire-missions"),
            None,
        )
        .await;
    assert_eq!(anonymous.0, StatusCode::UNAUTHORIZED);
    suite.pool().close().await;
}
