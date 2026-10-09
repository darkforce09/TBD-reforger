//! Deployments of approved artifacts, through the real routes: a same-terrain transition restarts
//! the scenario in the running game, and an in-game deployment request needs a linked platform
//! administrator.

use crate::mission_artifact_support;

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use mission_artifact_support::{MissionFixture, machine, refusal_code, uuid_of};

const SUITE: &str = "mission_deployment_transitions";
const EVERON: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

async fn artifact_modpack(f: &MissionFixture, artifact: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT modpack_id FROM mission_artifacts WHERE id = $1")
        .bind(artifact)
        .fetch_one(f.pool())
        .await
        .expect("the read of mission_artifacts returns a row")
}

/// A server able to run `artifact`: it requires the modpack the artifact compiled against.
async fn host_for(f: &MissionFixture, name: &str, artifact: Uuid) -> Uuid {
    let modpack = artifact_modpack(f, artifact).await;
    f.register_server(name, modpack).await
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
    .expect("the read of mission_deployments returns a row")
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
