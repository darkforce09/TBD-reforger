//! Registered servers, machine credentials and runtime sessions for the fleet suites, driven
//! through the real administrator and game-runtime routes of an `event_eligibility_support`
//! fixture.
//!
//! Compiled into each suite that writes `mod fleet_support;` next to
//! `mod event_eligibility_support;`; it adds no test binary.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::event_eligibility_support::{Actor, Fixture};

/// A machine caller: the credential secret presented as the bearer.
pub fn machine(secret: &str) -> Actor {
    Actor {
        id: String::new(),
        token: secret.to_owned(),
    }
}

pub async fn register_server(f: &Fixture, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1'::inet, 2001, true) RETURNING id",
    )
    .bind(name)
    .fetch_one(f.pool())
    .await
    .unwrap()
}

pub async fn issue(f: &Fixture, server: Uuid, executor: &str, label: &str) -> (StatusCode, Value) {
    f.call(
        &f.admin,
        "POST",
        &format!("/api/v1/servers/{server}/credentials"),
        Some(json!({ "executor_kind": executor, "label": label })),
    )
    .await
}

/// Issue a credential and return its secret.
pub async fn credential(f: &Fixture, server: Uuid, executor: &str) -> String {
    let (status, body) = issue(f, server, executor, &format!("{executor} credential")).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["secret"].as_str().unwrap().to_owned()
}

pub async fn revoke(
    f: &Fixture,
    server: Uuid,
    credential: &str,
    reason: &str,
) -> (StatusCode, Value) {
    f.call(
        &f.admin,
        "DELETE",
        &format!("/api/v1/servers/{server}/credentials/{credential}?reason={reason}"),
        None,
    )
    .await
}

pub async fn start_session(f: &Fixture, secret: &str) -> (StatusCode, Value) {
    f.call(
        &machine(secret),
        "POST",
        "/api/v1/game-runtime/sessions",
        None,
    )
    .await
}

/// Start a session and return `(runtime_session_id, generation)`.
pub async fn session(f: &Fixture, secret: &str) -> (Uuid, i64) {
    let (status, body) = start_session(f, secret).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (
        body["runtime_session_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        body["generation"].as_i64().unwrap(),
    )
}

/// One heartbeat of `session` carrying `reading` fields.
pub async fn heartbeat(
    f: &Fixture,
    secret: &str,
    session: Uuid,
    generation: i64,
    sequence: i64,
    reading: Value,
) -> (StatusCode, Value) {
    let mut body = json!({ "generation": generation, "sequence": sequence });
    for (key, value) in reading.as_object().unwrap() {
        body[key] = value.clone();
    }
    f.call(
        &machine(secret),
        "POST",
        &format!("/api/v1/game-runtime/sessions/{session}/heartbeats"),
        Some(body),
    )
    .await
}

pub fn refusal_code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

/// Bind the fixture event to `server`, as an administrator scheduling it there would.
pub async fn bind_event(f: &Fixture, server: Uuid) {
    sqlx::query("UPDATE events SET server_id = $2 WHERE id = $1")
        .bind(f.event)
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();
}

/// Ask the runtime session to deploy `arma_id` as `life` into a fixture slot.
pub async fn deploy(
    f: &Fixture,
    secret: &str,
    session: Uuid,
    (mission, slot): (usize, usize),
    arma_id: &str,
    life: &str,
) -> (StatusCode, Value) {
    f.call(
        &machine(secret),
        "POST",
        &format!("/api/v1/game-runtime/sessions/{session}/deployments"),
        Some(json!({
            "event_mission_id": f.missions[mission],
            "orbat_slot_id": f.slots[mission][slot],
            "arma_id": arma_id,
            "player_life_id": life,
        })),
    )
    .await
}

pub async fn end_life(
    f: &Fixture,
    secret: &str,
    session: Uuid,
    occupancy: &str,
) -> (StatusCode, Value) {
    f.call(
        &machine(secret),
        "POST",
        &format!("/api/v1/game-runtime/sessions/{session}/deployments/{occupancy}/end"),
        None,
    )
    .await
}

/// `(ended, end_reason)` of one occupancy.
pub async fn occupancy_state(f: &Fixture, occupancy: &str) -> (bool, Option<String>) {
    sqlx::query_as(
        "SELECT ended_at IS NOT NULL, end_reason FROM live_slot_occupancies WHERE id = $1::uuid",
    )
    .bind(occupancy)
    .fetch_one(f.pool())
    .await
    .unwrap()
}

/// The Arma identity `common::access_token` links for fixture members.
pub fn linked_arma(actor: &Actor) -> String {
    format!("test-arma:{}", actor.id)
}

/// Record a deployment of fixture mission `index` on `server` with every seat of its event
/// mission bound to a compiled slot uid (`uid-<orbat slot id>`), as a validated deployment
/// records it, and answer the artifact and its document digest. The artifact's bytes are a
/// placeholder: the suites using it exercise occupancy, rosters and sessions, not compilation,
/// which `mission_deployment_transitions.rs` drives through the real routes.
pub async fn seed_deployment(f: &Fixture, server: Uuid, index: usize) -> (Uuid, String) {
    let mission = f.catalog_mission(index).await;
    let mut tx = f.pool().begin().await.unwrap();
    let version: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, created_by, created_at)
         VALUES ($1, '0.0.0-fixture.' || replace(gen_random_uuid()::text, '-', ''), '{}'::jsonb, $2, now())
         RETURNING id",
    )
    .bind(mission)
    .bind(&f.admin.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let (artifact, sha256): (Uuid, String) = sqlx::query_as(
        "INSERT INTO mission_artifacts (mission_id, mission_version_id, version_payload_sha256,
             metadata, metadata_sha256, catalog_sha256, compiler_version, schema_version, terrain,
             document, document_sha256, document_bytes, artifact_digest, created_by)
         VALUES ($1, $2, repeat('a', 64), '{}'::jsonb, repeat('b', 64), repeat('c', 64),
             'fixture', '1', 'everon', convert_to('{\"slots\":[]}', 'UTF8'),
             encode(sha256(convert_to('{\"slots\":[]}', 'UTF8')), 'hex'), 12,
             encode(sha256(convert_to(gen_random_uuid()::text, 'UTF8')), 'hex'), $3)
         RETURNING id, document_sha256",
    )
    .bind(mission)
    .bind(version)
    .bind(&f.admin.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let command: Uuid = sqlx::query_scalar(
        "INSERT INTO fleet_commands (server_id, executor_kind, action, idempotent, process_changing,
             requested_by, expires_at, state, executing_at, finished_at)
         VALUES ($1, 'mod_runtime', 'load_mission', false, true, $2, now() + interval '1 hour',
             'succeeded', now(), now())
         RETURNING id",
    )
    .bind(server)
    .bind(&f.admin.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let deployment: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_deployments (server_id, mission_id, artifact_id, event_mission_id,
             terrain_key, scenario_id, transition, fleet_command_id, requested_by, requested_via,
             deadline_at)
         VALUES ($1, $2, $3, $4, 'everon', '{0000000000000000}Missions/Fixture.conf',
             'scenario_restart', $5, $6, 'web', now() + interval '1 hour')
         RETURNING id",
    )
    .bind(server)
    .bind(mission)
    .bind(artifact)
    .bind(f.missions[index])
    .bind(command)
    .bind(&f.admin.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mission_deployment_slots (deployment_id, orbat_slot_id, slot_uid)
         SELECT $1, id, 'uid-' || id FROM orbat_slots WHERE event_mission_id = $2",
    )
    .bind(deployment)
    .bind(f.missions[index])
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (artifact, sha256)
}

/// Start a session that reports running `artifact`; answers `(runtime_session_id, generation)`.
pub async fn running_session(
    f: &Fixture,
    secret: &str,
    (artifact, sha256): &(Uuid, String),
) -> (Uuid, i64) {
    let (status, body) = f
        .call(
            &machine(secret),
            "POST",
            "/api/v1/game-runtime/sessions",
            Some(json!({ "loaded_artifact_id": artifact, "loaded_artifact_sha256": sha256 })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (
        body["runtime_session_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        body["generation"].as_i64().unwrap(),
    )
}

/// A registered server bound to the fixture event, running fixture mission 0 through a recorded
/// deployment: its runtime credential and an open session that reports the deployed artifact.
pub async fn event_runtime(f: &Fixture) -> (Uuid, String, Uuid) {
    let server = register_server(f, "Event host").await;
    bind_event(f, server).await;
    let secret = credential(f, server, "mod_runtime").await;
    let deployed = seed_deployment(f, server, 0).await;
    let (live, _) = running_session(f, &secret, &deployed).await;
    (server, secret, live)
}

/// The artifact the server's recorded deployment runs, as [`seed_deployment`] answered it.
pub async fn deployed_artifact(f: &Fixture, server: Uuid) -> (Uuid, String) {
    sqlx::query_as(
        "SELECT d.artifact_id, a.document_sha256 FROM mission_deployments d
         JOIN mission_artifacts a ON a.id = d.artifact_id
         WHERE d.server_id = $1 ORDER BY d.requested_at DESC LIMIT 1",
    )
    .bind(server)
    .fetch_one(f.pool())
    .await
    .unwrap()
}
