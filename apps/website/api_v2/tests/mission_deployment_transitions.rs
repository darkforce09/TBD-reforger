//! Deployments of approved artifacts, through the real routes: a selection is validated before
//! anything is persisted, the transition follows the terrain the server runs, a runtime reads only
//! what is deployed to its server, roster derivation and seat authorization follow the artifact the
//! runtime reported loading, and a deployment is confirmed only by a later runtime session
//! reporting its exact artifact — every other outcome fails it observably.

mod common;
mod mission_artifact_support;

use axum::http::{StatusCode, header};
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments;

use mission_artifact_support::{MissionFixture, machine, refusal_code, sha256_hex, uuid_of};

const SUITE: &str = "mission_deployment_transitions";
const EVERON: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";
const ARLAND: &str = "{1111222233334444}Missions/TBD_Arland.conf";

async fn artifact_modpack(f: &MissionFixture, artifact: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT modpack_id FROM mission_artifacts WHERE id = $1")
        .bind(artifact)
        .fetch_one(f.pool())
        .await
        .unwrap()
}

/// A server able to run `artifact`: it requires the modpack the artifact compiled against.
async fn host_for(f: &MissionFixture, name: &str, artifact: Uuid) -> Uuid {
    let modpack = artifact_modpack(f, artifact).await;
    f.register_server(name, modpack).await
}

/// An approved mission on Arland, whose artifact runs only on a registered Arland scenario.
async fn approved_on_arland(f: &MissionFixture, title: &str) -> (Uuid, Uuid) {
    approved_on_terrain(f, title, json!({ "terrain": "arland" })).await
}

/// An approved mission on a custom terrain no other test registers a scenario for.
async fn approved_on_unregistered_terrain(f: &MissionFixture, title: &str) -> (Uuid, Uuid) {
    let name = format!("unregistered_{}", Uuid::new_v4().simple());
    approved_on_terrain(
        f,
        title,
        json!({ "terrain": "custom", "custom_terrain_name": name }),
    )
    .await
}

async fn approved_on_terrain(f: &MissionFixture, title: &str, terrain: Value) -> (Uuid, Uuid) {
    let mut body = json!({ "title": title, "game_mode": "pve_coop", "max_players": 16 });
    for (key, value) in terrain.as_object().unwrap() {
        body[key] = value.clone();
    }
    let (status, created) = f
        .call(Some(&f.author), "POST", "/api/v1/missions", Some(body))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let mission = uuid_of(&created["id"]);
    f.save(mission, "0.2.0", common::COMPILABLE_EDITOR_PAYLOAD)
        .await;
    let artifact = f.submit(mission).await;
    let (status, body) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (mission, artifact)
}

async fn session_id(f: &MissionFixture, secret: &str, loaded: Option<(Uuid, &str)>) -> Uuid {
    let (status, body) = f.start_session(secret, loaded).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    uuid_of(&body["runtime_session_id"])
}

async fn claim(f: &MissionFixture, secret: &str, session: Option<Uuid>) -> (StatusCode, Value) {
    let body = session.map_or(
        json!({}),
        |session| json!({ "runtime_session_id": session }),
    );
    f.call(
        Some(&machine(secret)),
        "POST",
        "/api/v1/fleet-executor/commands/claim",
        Some(body),
    )
    .await
}

async fn deployments_on(f: &MissionFixture, servers: &[Uuid]) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM mission_deployments WHERE server_id = ANY($1)),
                (SELECT count(*) FROM fleet_commands WHERE server_id = ANY($1))",
    )
    .bind(servers)
    .fetch_one(f.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn artifact_consumers_runtime_reads_only_what_is_deployed_to_its_server() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("Consumer").await;
    let server = host_for(&f, "Consumer host", artifact).await;
    let runtime = f.credential(server, "mod_runtime").await;
    let current = "/api/v1/game-runtime/deployment";

    let (status, body) = f.call(Some(&machine(&runtime)), "GET", current, None).await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::NOT_FOUND, "NO_DEPLOYMENT")
    );
    let deployment = f.deploy(server, mission, artifact, None).await;
    assert_eq!(deployment["state"], "requested");
    assert_eq!(
        deployment["transition"], "host_restart",
        "no runtime reports a terrain yet"
    );

    let sha256 = f.artifact_sha256(artifact).await;
    let (status, running) = f.call(Some(&machine(&runtime)), "GET", current, None).await;
    assert_eq!(status, StatusCode::OK, "{running}");
    assert_eq!(running["deployment_id"], deployment["id"]);
    assert_eq!(uuid_of(&running["artifact_id"]), artifact);
    assert_eq!(running["artifact_sha256"], sha256.as_str());
    assert_eq!(running["scenario_id"], EVERON);
    assert_eq!(running["terrain_key"], "everon");

    let bytes_route = |artifact: Uuid| format!("/api/v1/game-runtime/artifacts/{artifact}");
    let (status, headers, bytes) = f
        .send(
            Some(&machine(&runtime)),
            "GET",
            &bytes_route(artifact),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sha256_hex(&bytes), sha256);
    assert_eq!(
        headers[header::ETAG].to_str().unwrap(),
        format!("\"{sha256}\"")
    );
    assert_eq!(headers["x-compile-diagnostics-count"], "0");

    let (_, elsewhere) = f.approved_mission("Deployed nowhere").await;
    let (status, body) = f
        .call(
            Some(&machine(&runtime)),
            "GET",
            &bytes_route(elsewhere),
            None,
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::FORBIDDEN, "ARTIFACT_NOT_DEPLOYED_HERE")
    );
    let foreign = host_for(&f, "Foreign host", artifact).await;
    let foreign_runtime = f.credential(foreign, "mod_runtime").await;
    let (status, body) = f
        .call(
            Some(&machine(&foreign_runtime)),
            "GET",
            &bytes_route(artifact),
            None,
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::FORBIDDEN, "ARTIFACT_NOT_DEPLOYED_HERE")
    );
    let agent = f.credential(server, "host_agent").await;
    assert_eq!(
        f.call(Some(&machine(&agent)), "GET", current, None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.call(None, "GET", current, None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // In-game administrators choose among approved artifacts this server can run.
    let (status, listed) = f
        .call(
            Some(&machine(&runtime)),
            "GET",
            "/api/v1/game-runtime/missions",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let offered: Vec<Uuid> = listed["missions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| uuid_of(&entry["artifact_id"]))
        .collect();
    assert!(
        offered.contains(&artifact) && offered.contains(&elsewhere),
        "{listed}"
    );
}

#[tokio::test]
async fn artifact_consumers_roster_and_seat_authorization_follow_the_loaded_artifact() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("Roster source").await;
    let server = host_for(&f, "Roster host", artifact).await;
    let runtime = f.credential(server, "mod_runtime").await;
    let (event, event_mission) = f.event_on(server, mission).await;
    let deployment = f
        .deploy(server, mission, artifact, Some(event_mission))
        .await;
    assert_eq!(deployment["bound_slots"], 1, "{deployment}");
    let seat: Uuid = sqlx::query_scalar("SELECT id FROM orbat_slots WHERE event_mission_id = $1")
        .bind(event_mission)
        .fetch_one(f.pool())
        .await
        .unwrap();

    let (status, roster) = f
        .call(
            Some(&machine(&runtime)),
            "GET",
            &format!("/api/v1/game-runtime/events/{event}/roster"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{roster}");
    assert_eq!(uuid_of(&roster["missionId"]), mission);
    assert_eq!(
        roster["slots"],
        json!([{ "eventMissionId": event_mission, "slotUid": "s1", "orbatSlotId": seat }])
    );

    let request = |life: &str| {
        json!({ "event_mission_id": event_mission, "orbat_slot_id": seat,
                "arma_id": format!("test-arma:{}", f.admin.id), "player_life_id": life })
    };
    let deploy_into =
        |session: Uuid| format!("/api/v1/game-runtime/sessions/{session}/deployments");
    // A runtime that loaded no artifact runs no event seat.
    let bare = session_id(&f, &runtime, None).await;
    let (status, refused) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            &deploy_into(bare),
            Some(request("life-1")),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{refused}");
    assert_eq!(
        (refused["decision"].as_str(), refused["reason"].as_str()),
        (Some("denied"), Some("SLOT_NOT_IN_LOADED_MISSION"))
    );
    // The runtime that reports the deployed artifact seats the player.
    let sha256 = f.artifact_sha256(artifact).await;
    let running = session_id(&f, &runtime, Some((artifact, &sha256))).await;
    let (status, allowed) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            &deploy_into(running),
            Some(request("life-2")),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{allowed}");
    assert_eq!(allowed["decision"], "allowed", "{allowed}");
    // A runtime that reports another artifact runs no seat of this deployment.
    let (_, other) = f.approved_mission("Other mission").await;
    let other_sha = f.artifact_sha256(other).await;
    let stranger = session_id(&f, &runtime, Some((other, &other_sha))).await;
    let (_, refused) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            &deploy_into(stranger),
            Some(request("life-3")),
        )
        .await;
    assert_eq!(refused["reason"], "SLOT_NOT_IN_LOADED_MISSION", "{refused}");
}

#[tokio::test]
async fn mission_transitions_selection_is_validated_before_anything_is_persisted() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("Validated").await;
    let server = host_for(&f, "Validated host", artifact).await;
    let mut touched = vec![server];

    let (status, _) = f
        .request_deployment(&f.author, server, mission, artifact, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "only administrators deploy");
    let (status, _) = f
        .request_deployment(&f.admin, Uuid::new_v4(), mission, artifact, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let inactive = host_for(&f, "Inactive host", artifact).await;
    touched.push(inactive);
    sqlx::query("UPDATE servers SET is_active = false WHERE id = $1")
        .bind(inactive)
        .execute(f.pool())
        .await
        .unwrap();
    let (status, body) = f
        .request_deployment(&f.admin, inactive, mission, artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "SERVER_INACTIVE")
    );

    let (pending, _) = f.compilable_mission("Still under review").await;
    let pending_artifact = f.submit(pending).await;
    let (status, body) = f
        .request_deployment(&f.admin, server, pending, pending_artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "ARTIFACT_NOT_APPROVED")
    );
    let (status, body) = f
        .request_deployment(&f.admin, server, mission, pending_artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "ARTIFACT_NOT_APPROVED"),
        "{body}"
    );

    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at)
         VALUES ('Other pack', '9.0.0', 0, false, now()) RETURNING id",
    )
    .fetch_one(f.pool())
    .await
    .unwrap();
    let other_pack = f.register_server("Other pack host", Some(modpack)).await;
    touched.push(other_pack);
    let (status, body) = f
        .request_deployment(&f.admin, other_pack, mission, artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::UNPROCESSABLE_ENTITY, "MODPACK_MISMATCH"),
        "{body}"
    );

    let (custom_mission, custom_artifact) =
        approved_on_unregistered_terrain(&f, "Custom terrain op").await;
    let (status, body) = f
        .request_deployment(&f.admin, server, custom_mission, custom_artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::UNPROCESSABLE_ENTITY, "TERRAIN_NOT_RUNNABLE"),
        "{body}"
    );
    assert!(
        body["details"]["terrain_key"]
            .as_str()
            .unwrap()
            .starts_with("unregistered_"),
        "{body}"
    );

    let elsewhere = host_for(&f, "Event elsewhere", artifact).await;
    touched.push(elsewhere);
    let (_, foreign_event_mission) = f.event_on(elsewhere, mission).await;
    let (status, body) = f
        .request_deployment(
            &f.admin,
            server,
            mission,
            artifact,
            Some(foreign_event_mission),
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "EVENT_MISSION_NOT_ON_SERVER"),
        "{body}"
    );

    // An ORBAT that does not correspond one to one with the artifact's slots is refused with
    // every unpaired seat and slot named.
    let (status, event) = f
        .call(
            Some(&f.admin),
            "POST",
            "/api/v1/events",
            Some(json!({ "start_time": "2027-11-02T00:00:00Z" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{event}");
    let event = uuid_of(&event["id"]);
    sqlx::query("UPDATE events SET server_id = $2 WHERE id = $1")
        .bind(event)
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();
    let orbat = json!([{ "faction": "BLUFOR", "callsign": "Alpha", "squad": "Wrong squad",
                         "slots": [{ "role": "SL" }, { "role": "Medic" }] }]);
    let (status, attached) = f
        .call(
            Some(&f.admin),
            "POST",
            &format!("/api/v1/events/{event}/missions"),
            Some(json!({ "mission_id": mission, "start_time": "2027-11-02T00:00:00Z", "orbat": orbat })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{attached}");
    let (status, body) = f
        .request_deployment(
            &f.admin,
            server,
            mission,
            artifact,
            Some(uuid_of(&attached["id"])),
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::UNPROCESSABLE_ENTITY, "ORBAT_ARTIFACT_MISMATCH"),
        "{body}"
    );
    assert_eq!(
        body["details"]["unbound_seats"].as_array().map(Vec::len),
        Some(2),
        "{body}"
    );
    assert_eq!(body["details"]["unseated_slots"], json!(["s1 (SL)"]));

    assert_eq!(
        deployments_on(&f, &touched).await,
        (0, 0),
        "a refused selection persists nothing"
    );
}

#[tokio::test]
async fn mission_transitions_same_terrain_restarts_the_scenario_in_the_runtime() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (first_mission, first) = f.approved_mission("First").await;
    let (second_mission, second) = f.approved_mission("Second").await;
    let server = host_for(&f, "Same terrain host", first).await;
    let (runtime, agent) = (
        f.credential(server, "mod_runtime").await,
        f.credential(server, "host_agent").await,
    );
    f.deploy(server, first_mission, first, None).await;
    let first_sha = f.artifact_sha256(first).await;
    let live = session_id(&f, &runtime, Some((first, &first_sha))).await;
    // The host restart the first deployment issued is not what this test examines.
    sqlx::query("UPDATE fleet_commands SET state = 'cancelled', finished_at = now(), failure_reason = 'test' WHERE server_id = $1")
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();

    let deployment = f.deploy(server, second_mission, second, None).await;
    assert_eq!(deployment["transition"], "scenario_restart", "{deployment}");
    assert_eq!(
        claim(&f, &agent, None).await.0,
        StatusCode::NO_CONTENT,
        "the host agent does not restart it"
    );
    let (status, claimed) = claim(&f, &runtime, Some(live)).await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["action"], "load_mission");
    assert_eq!(
        claimed["arguments"],
        json!({ "deployment_id": deployment["id"], "artifact_id": second,
                "artifact_sha256": f.artifact_sha256(second).await, "runtime_session_id": live })
    );
    assert_eq!(claimed["command_id"], deployment["fleet_command_id"]);
}

/// A scenario restart belongs to the runtime whose terrain was validated: when that session
/// ends before the command is claimed, the command fails and so does the deployment, instead of
/// reaching a runtime that may run another terrain.
#[tokio::test]
async fn mission_transitions_scenario_restart_of_an_ended_session_fails_the_deployment() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (first_mission, first) = f.approved_mission("Ended first").await;
    let (second_mission, second) = f.approved_mission("Ended second").await;
    let server = host_for(&f, "Ended session host", first).await;
    let runtime = f.credential(server, "mod_runtime").await;
    f.deploy(server, first_mission, first, None).await;
    let first_sha = f.artifact_sha256(first).await;
    session_id(&f, &runtime, Some((first, &first_sha))).await;
    sqlx::query("UPDATE fleet_commands SET state = 'cancelled', finished_at = now(), failure_reason = 'test' WHERE server_id = $1")
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();
    let deployment = f.deploy(server, second_mission, second, None).await;
    assert_eq!(deployment["transition"], "scenario_restart");
    // The runtime restarts on its own before claiming: a new session, nothing loaded yet.
    let restarted = session_id(&f, &runtime, None).await;
    assert_eq!(
        claim(&f, &runtime, Some(restarted)).await.0,
        StatusCode::NO_CONTENT
    );
    let failed = f.deployment(server, &deployment).await;
    assert_eq!(failed["state"], "failed", "{failed}");
    assert_eq!(failed["fleet_command_state"], "failed");
    assert!(
        failed["failure_reason"]
            .as_str()
            .unwrap()
            .contains("the runtime session the command was issued against has ended"),
        "{failed}"
    );
}

#[tokio::test]
async fn mission_transitions_cross_terrain_restarts_the_host_on_the_new_terrain_scenario() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    f.register_scenario("arland", ARLAND).await;
    let (everon_mission, everon) = f.approved_mission("Everon op").await;
    let (arland_mission, arland) = approved_on_arland(&f, "Arland op").await;
    let server = host_for(&f, "Cross terrain host", everon).await;
    let (runtime, agent) = (
        f.credential(server, "mod_runtime").await,
        f.credential(server, "host_agent").await,
    );
    let first = f.deploy(server, everon_mission, everon, None).await;
    let everon_sha = f.artifact_sha256(everon).await;
    let live = session_id(&f, &runtime, Some((everon, &everon_sha))).await;
    assert_eq!(f.deployment(server, &first).await["state"], "confirmed");
    sqlx::query("UPDATE fleet_commands SET state = 'cancelled', finished_at = now(), failure_reason = 'test' WHERE server_id = $1")
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();

    let deployment = f.deploy(server, arland_mission, arland, None).await;
    assert_eq!(deployment["transition"], "host_restart", "{deployment}");
    assert_eq!(deployment["scenario_id"], ARLAND);
    assert_eq!(
        claim(&f, &runtime, Some(live)).await.0,
        StatusCode::NO_CONTENT
    );
    let (status, claimed) = claim(&f, &agent, None).await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["action"], "restart_with_mission");
    assert_eq!(claimed["arguments"]["scenario_id"], ARLAND);
    assert_eq!(uuid_of(&claimed["arguments"]["artifact_id"]), arland);
    assert_eq!(claimed["arguments"]["deployment_id"], deployment["id"]);
}

#[tokio::test]
async fn mission_transitions_one_deployment_in_flight_per_server_and_operators_cannot_bypass_it() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("One at a time").await;
    let server = host_for(&f, "Busy host", artifact).await;
    let first = f.deploy(server, mission, artifact, None).await;
    let (status, body) = f
        .request_deployment(&f.admin, server, mission, artifact, None)
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "DEPLOYMENT_IN_PROGRESS")
    );
    assert_eq!(body["details"]["deployment_id"], first["id"]);

    let raced = host_for(&f, "Raced host", artifact).await;
    let (a, b) = tokio::join!(
        f.request_deployment(&f.admin, raced, mission, artifact, None),
        f.request_deployment(&f.admin, raced, mission, artifact, None)
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::ACCEPTED, StatusCode::CONFLICT],
        "{a:?} {b:?}"
    );
    assert_eq!(deployments_on(&f, &[raced]).await, (1, 1));

    // The deployment actions are issued by deployments only.
    for action in ["load_mission", "restart_with_mission"] {
        let (status, body) = f
            .call(
                Some(&f.admin),
                "POST",
                &format!("/api/v1/servers/{server}/commands"),
                Some(json!({ "action": action })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{action}: {body}");
    }
}

#[tokio::test]
async fn mission_transitions_in_game_request_needs_a_linked_platform_administrator() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("In-game choice").await;
    let relayed = |arma: String| json!({ "mission_id": mission, "artifact_id": artifact, "requested_by_arma_id": arma });
    let server = host_for(&f, "In-game host", artifact).await;
    let runtime = f.credential(server, "mod_runtime").await;
    let route = "/api/v1/game-runtime/deployments";

    let (status, body) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            route,
            Some(relayed(format!("test-arma:{}", f.author.id))),
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::FORBIDDEN, "NOT_AN_ADMINISTRATOR"),
        "{body}"
    );
    let (status, body) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            route,
            Some(relayed("unlinked-arma".into())),
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::FORBIDDEN, "IDENTITY_NOT_LINKED"),
        "{body}"
    );
    let (status, body) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            route,
            Some(json!({ "mission_id": mission })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(deployments_on(&f, &[server]).await, (0, 0));

    let (status, accepted) = f
        .call(
            Some(&machine(&runtime)),
            "POST",
            route,
            Some(relayed(format!("  test-arma:{}  ", f.admin.id))),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{accepted}");
    assert_eq!(accepted["requested_via"], "game_runtime");
    assert_eq!(accepted["requested_by"], f.admin.id.as_str());
    assert_eq!(
        uuid_of(&accepted["server_id"]),
        server,
        "a runtime deploys to its own server only"
    );
    let agent = f.credential(server, "host_agent").await;
    let (status, _) = f
        .call(
            Some(&machine(&agent)),
            "POST",
            route,
            Some(relayed(format!("test-arma:{}", f.admin.id))),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn deployment_confirmation_requires_the_exact_artifact_from_a_later_session() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("Confirmed").await;
    let (_, other) = f.approved_mission("Other bytes").await;
    let (sha256, other_sha) = (
        f.artifact_sha256(artifact).await,
        f.artifact_sha256(other).await,
    );

    // A report that predates the request decides nothing; a later one confirms.
    let server = host_for(&f, "Confirming host", artifact).await;
    let runtime = f.credential(server, "mod_runtime").await;
    session_id(&f, &runtime, Some((artifact, &sha256))).await;
    let deployment = f.deploy(server, mission, artifact, None).await;
    assert_eq!(
        f.deployment(server, &deployment).await["state"],
        "requested"
    );
    let confirming = session_id(&f, &runtime, Some((artifact, &sha256))).await;
    let confirmed = f.deployment(server, &deployment).await;
    assert_eq!(confirmed["state"], "confirmed", "{confirmed}");
    assert_eq!(
        uuid_of(&confirmed["confirmed_runtime_session_id"]),
        confirming
    );
    assert!(confirmed.get("finished_at").is_some() && confirmed.get("failure_reason").is_none());

    // Another artifact, or the right artifact with other bytes, fails the deployment.
    for (name, reported) in [
        ("Wrong artifact host", (other, other_sha.as_str())),
        ("Wrong bytes host", (artifact, other_sha.as_str())),
    ] {
        let host = host_for(&f, name, artifact).await;
        let secret = f.credential(host, "mod_runtime").await;
        let deployment = f.deploy(host, mission, artifact, None).await;
        session_id(&f, &secret, Some(reported)).await;
        let failed = f.deployment(host, &deployment).await;
        assert_eq!(failed["state"], "failed", "{name}: {failed}");
        assert!(
            failed["failure_reason"]
                .as_str()
                .unwrap()
                .contains("instead of"),
            "{failed}"
        );
        // The failure is observable and the server takes a new deployment.
        f.deploy(host, mission, artifact, None).await;
    }
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action IN ('mission.deployment_confirmed', 'mission.deployment_failed')
         AND target_type = 'mission_deployment'",
    )
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert!(audited >= 3, "every settlement is audited");
}

#[tokio::test]
async fn deployment_confirmation_partial_transitions_and_failed_commands_fail_observably() {
    let f = MissionFixture::new(SUITE).await;
    f.register_scenario("everon", EVERON).await;
    let (mission, artifact) = f.approved_mission("Partial").await;

    // No runtime reports the artifact before the deadline: a partial transition.
    let late = host_for(&f, "Late host", artifact).await;
    let deployment = f.deploy(late, mission, artifact, None).await;
    sqlx::query("UPDATE mission_deployments SET requested_at = now() - interval '2 hours', deadline_at = now() - interval '1 hour' WHERE id = $1::uuid")
        .bind(deployment["id"].as_str().unwrap())
        .execute(f.pool())
        .await
        .unwrap();
    assert!(reconcile_mission_deployments(f.pool()).await.unwrap() >= 1);
    let failed = f.deployment(late, &deployment).await;
    assert_eq!(failed["state"], "failed");
    assert!(
        failed["failure_reason"]
            .as_str()
            .unwrap()
            .starts_with("partial transition"),
        "{failed}"
    );

    // The host agent reports that the restart failed.
    let broken = host_for(&f, "Broken host", artifact).await;
    let agent = f.credential(broken, "host_agent").await;
    let deployment = f.deploy(broken, mission, artifact, None).await;
    let (status, claimed) = claim(&f, &agent, None).await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    let command = claimed["command_id"].as_str().unwrap();
    let token = claimed["fencing_token"].clone();
    let report = |stage: &'static str, body: Value| {
        let uri = format!("/api/v1/fleet-executor/commands/{command}/{stage}");
        let caller = machine(&agent);
        let f = &f;
        async move { f.call(Some(&caller), "POST", &uri, Some(body)).await }
    };
    assert_eq!(
        report("executing", json!({ "fencing_token": token }))
            .await
            .0,
        StatusCode::OK
    );
    let (status, _) = report(
        "result",
        json!({ "fencing_token": token, "succeeded": false, "failure_reason": "the unit did not come back active" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let failed = f.deployment(broken, &deployment).await;
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["fleet_command_state"], "failed");
    assert!(
        failed["failure_reason"]
            .as_str()
            .unwrap()
            .contains("the unit did not come back active"),
        "{failed}"
    );

    // A queued deployment is cancelled with its command; a claimed one no longer can be.
    let cancelled_host = host_for(&f, "Cancelled host", artifact).await;
    let deployment = f.deploy(cancelled_host, mission, artifact, None).await;
    let cancel = |server: Uuid, deployment: &Value| {
        format!(
            "/api/v1/servers/{server}/deployments/{}/cancel",
            deployment["id"].as_str().unwrap()
        )
    };
    let (status, cancelled) = f
        .call(
            Some(&f.admin),
            "POST",
            &cancel(cancelled_host, &deployment),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(
        (
            cancelled["state"].as_str(),
            cancelled["fleet_command_state"].as_str()
        ),
        (Some("cancelled"), Some("cancelled"))
    );
    let (status, body) = f
        .call(
            Some(&f.admin),
            "POST",
            &cancel(cancelled_host, &deployment),
            None,
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "DEPLOYMENT_NOT_IN_FLIGHT")
    );
    let claimed_host = host_for(&f, "Claimed host", artifact).await;
    let claimed_agent = f.credential(claimed_host, "host_agent").await;
    let deployment = f.deploy(claimed_host, mission, artifact, None).await;
    assert_eq!(claim(&f, &claimed_agent, None).await.0, StatusCode::OK);
    let (status, body) = f
        .call(
            Some(&f.admin),
            "POST",
            &cancel(claimed_host, &deployment),
            None,
        )
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "COMMAND_NOT_CANCELLABLE"),
        "{body}"
    );
    assert_eq!(
        f.deployment(claimed_host, &deployment).await["state"],
        "requested"
    );
    let (status, _) = f
        .call(
            Some(&f.author),
            "POST",
            &cancel(claimed_host, &deployment),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, page) = f
        .call(
            Some(&f.admin),
            "GET",
            &format!("/api/v1/servers/{broken}/deployments"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["state"], "failed");
}

#[tokio::test]
async fn deployment_confirmation_session_start_refuses_unknown_or_partial_reports() {
    let f = MissionFixture::new(SUITE).await;
    let (_, artifact) = f.approved_mission("Reported").await;
    let server = host_for(&f, "Reporting host", artifact).await;
    let runtime = f.credential(server, "mod_runtime").await;
    let sha256 = f.artifact_sha256(artifact).await;
    let (status, body) = f
        .start_session(&runtime, Some((Uuid::new_v4(), &sha256)))
        .await;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::UNPROCESSABLE_ENTITY, "UNKNOWN_ARTIFACT"),
        "{body}"
    );
    for body in [
        json!({ "loaded_artifact_id": artifact }),
        json!({ "loaded_artifact_sha256": sha256 }),
        json!({ "loaded_artifact_id": artifact, "loaded_artifact_sha256": sha256.to_uppercase() }),
        json!({ "loaded_artifact_id": artifact, "loaded_artifact_sha256": sha256, "extra": 1 }),
    ] {
        let (status, response) = f
            .call(
                Some(&machine(&runtime)),
                "POST",
                "/api/v1/game-runtime/sessions",
                Some(body.clone()),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {response}");
    }
    let open: i64 =
        sqlx::query_scalar("SELECT count(*) FROM server_runtime_sessions WHERE server_id = $1")
            .bind(server)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(open, 0, "a refused report starts no session");
    let (status, started) = f.start_session(&runtime, Some((artifact, &sha256))).await;
    assert_eq!(status, StatusCode::CREATED, "{started}");
}
