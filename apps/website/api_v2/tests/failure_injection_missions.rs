//! Failure injection at the mission review decision and the deployment request: a review decision
//! failing before its commit, and a deployment request failing before and after its commit, from
//! the website and from an in-game relay.
//!
//! **Role:** proves each mission failpoint's documented outcome through the real router. A
//! decision failing before its commit leaves the review pending, the mission unapproved and no
//! comment, audit row or pending publication, for approval and rejection alike, and a clean retry
//! decides it. A deployment request failing before its commit leaves no deployment, seat binding,
//! fleet command, audit row or pending publication, and a clean retry deploys. A deployment request
//! whose answer is lost after its commit is recorded once, and the retry is refused as
//! `DEPLOYMENT_IN_PROGRESS` naming the committed deployment, which the caller reads back.
//! **Position:** its own test binary over `tests/common` and `tests/mission_artifact_support`
//! (accounts, missions, approvals, servers, credentials) and `tests/failpoint_and_race_support`
//! (suite lock, arming, persisted-state checks).
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case takes first; each case owns a fresh fixture, missions and servers.
//! **Invariants:** every case leaves nothing armed; whole-table row counts are compared only while
//! the suite lock is held, so no other case of this binary writes in between.

mod common;
mod failpoint_and_race_support;
mod mission_artifact_support;

use axum::http::StatusCode;
use failpoint_and_race_support::{
    Failpoint, FailpointArming, RowCounts, assert_injected_failure, audit_evidence,
    check_review_decided_once, lock_suite,
};
use mission_artifact_support::{MissionFixture, machine, refusal_code, uuid_of};
use serde_json::{Value, json};
use uuid::Uuid;

const SUITE: &str = "failure_injection_missions";

/// The Everon mission header a deployment's host restart boots.
const EVERON: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

/// The tables a review decision writes by insertion; a failed decision leaves each unchanged.
const DECISION_TABLES: [&str; 3] = [
    "mission_review_comments",
    "audit_logs",
    "audit_publication_pending",
];

/// The tables a deployment request writes; a failed request leaves each unchanged.
const DEPLOYMENT_TABLES: [&str; 5] = [
    "mission_deployments",
    "mission_deployment_slots",
    "fleet_commands",
    "audit_logs",
    "audit_publication_pending",
];

/// The mission's status and approved artifact.
async fn mission_standing(fixture: &MissionFixture, mission: Uuid) -> (String, Option<Uuid>) {
    sqlx::query_as("SELECT status::text, approved_artifact_id FROM missions WHERE id = $1")
        .bind(mission)
        .fetch_one(fixture.pool())
        .await
        .expect("read the mission")
}

/// The states of the mission's reviews, oldest first.
async fn review_states(fixture: &MissionFixture, mission: Uuid) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT state FROM mission_reviews WHERE mission_id = $1 ORDER BY submitted_at, id",
    )
    .bind(mission)
    .fetch_all(fixture.pool())
    .await
    .expect("read the reviews")
}

#[tokio::test]
async fn failure_injection_review_decision_before_commit_rolls_back_approval_and_rejection() {
    let suite = lock_suite().await;
    let fixture = MissionFixture::new(SUITE).await;
    let pool = fixture.pool();
    let (mission, _) = fixture.compilable_mission("Injected review").await;
    let artifact = fixture.submit(mission).await;
    let before = RowCounts::capture(pool, &DECISION_TABLES).await;
    {
        let guard = suite.fail(Failpoint::ReviewDecisionBeforeCommit);
        let (status, body) = fixture
            .approve(mission, artifact, Some("Keep the AO"))
            .await;
        assert_injected_failure(status, &body, Failpoint::ReviewDecisionBeforeCommit);
        let (status, body) = fixture
            .reject(mission, artifact, "Rework the briefing")
            .await;
        assert_injected_failure(status, &body, Failpoint::ReviewDecisionBeforeCommit);
        assert_eq!(
            guard.arrivals(),
            2,
            "approval and rejection reach the point"
        );
    }
    before.check_unchanged(pool).await.unwrap();
    assert_eq!(
        mission_standing(&fixture, mission).await,
        ("pending_approval".to_owned(), None)
    );
    assert_eq!(review_states(&fixture, mission).await, ["pending"]);
    assert_eq!(fixture.audits("mission.approve", mission).await, 0);
    assert_eq!(fixture.audits("mission.reject", mission).await, 0);
    check_review_decided_once(pool, mission).await.unwrap();

    // The clean retry decides the same review once.
    let (status, body) = fixture.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        mission_standing(&fixture, mission).await,
        ("live".to_owned(), Some(artifact))
    );
    assert_eq!(review_states(&fixture, mission).await, ["approved"]);
    let approval = audit_evidence(pool, "mission.approve", &mission.to_string()).await;
    assert_eq!(
        (approval.rows, approval.pending + approval.published),
        (1, 1),
        "{approval:?}"
    );
    check_review_decided_once(pool, mission).await.unwrap();
}

/// An approved Everon mission and a server able to run it: `(mission, artifact, server)`.
async fn deployable(fixture: &MissionFixture, title: &str) -> (Uuid, Uuid, Uuid) {
    fixture.register_scenario("everon", EVERON).await;
    let (mission, artifact) = fixture.approved_mission(title).await;
    let modpack: Option<Uuid> =
        sqlx::query_scalar("SELECT modpack_id FROM mission_artifacts WHERE id = $1")
            .bind(artifact)
            .fetch_one(fixture.pool())
            .await
            .expect("read the artifact's modpack");
    let server = fixture
        .register_server(&format!("{title} host"), modpack)
        .await;
    (mission, artifact, server)
}

/// The deployments of `server`: `(id, requested_via)`, oldest first.
async fn deployments_of(fixture: &MissionFixture, server: Uuid) -> Vec<(Uuid, String)> {
    sqlx::query_as(
        "SELECT id, requested_via FROM mission_deployments WHERE server_id = $1
         ORDER BY requested_at, id",
    )
    .bind(server)
    .fetch_all(fixture.pool())
    .await
    .expect("read the server's deployments")
}

async fn fleet_commands_of(fixture: &MissionFixture, server: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM fleet_commands WHERE server_id = $1")
        .bind(server)
        .fetch_one(fixture.pool())
        .await
        .expect("count the server's fleet commands")
}

/// Exactly one committed `mission.deployment_requested` audit row of `deployment`.
async fn assert_requested_once(fixture: &MissionFixture, deployment: Uuid) {
    let audits = audit_evidence(
        fixture.pool(),
        "mission.deployment_requested",
        &deployment.to_string(),
    )
    .await;
    assert_eq!(
        (audits.rows, audits.pending + audits.published),
        (1, 1),
        "{audits:?}"
    );
}

#[tokio::test]
async fn failure_injection_deployment_request_before_commit_rolls_back_and_a_retry_deploys() {
    let suite = lock_suite().await;
    let fixture = MissionFixture::new(SUITE).await;
    let pool = fixture.pool();
    let (mission, artifact, server) = deployable(&fixture, "Injected deployment").await;
    let before = RowCounts::capture(pool, &DEPLOYMENT_TABLES).await;
    {
        let guard = suite.fail(Failpoint::DeploymentRequestBeforeCommit);
        let (status, body) = fixture
            .request_deployment(&fixture.admin, server, mission, artifact, None)
            .await;
        assert_injected_failure(status, &body, Failpoint::DeploymentRequestBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    before.check_unchanged(pool).await.unwrap();
    assert!(deployments_of(&fixture, server).await.is_empty());
    assert_eq!(fleet_commands_of(&fixture, server).await, 0);

    // The clean retry records the deployment, its fleet command and its audit row once.
    let deployment = fixture.deploy(server, mission, artifact, None).await;
    let id = uuid_of(&deployment["id"]);
    assert_eq!(uuid_of(&deployment["artifact_id"]), artifact);
    assert_eq!(
        deployments_of(&fixture, server).await,
        [(id, "web".to_owned())]
    );
    assert_eq!(fleet_commands_of(&fixture, server).await, 1);
    assert_requested_once(&fixture, id).await;
}

/// After a request whose answer an armed after-commit point lost: the deployment is recorded
/// once, and the retry is refused as in progress, naming it, and changes nothing.
async fn assert_lost_answer_recovers(
    fixture: &MissionFixture,
    server: Uuid,
    artifact: Uuid,
    via: &str,
    retry: (StatusCode, Value),
) {
    let recorded = deployments_of(fixture, server).await;
    let [(deployment, requested_via)] = recorded.as_slice() else {
        panic!("the lost answer's deployment is recorded once: {recorded:?}");
    };
    assert_eq!(requested_via, via);
    assert_eq!(fleet_commands_of(fixture, server).await, 1);
    assert_requested_once(fixture, *deployment).await;

    let (status, body) = retry;
    assert_eq!(
        (status, refusal_code(&body)),
        (StatusCode::CONFLICT, "DEPLOYMENT_IN_PROGRESS"),
        "{body}"
    );
    assert_eq!(uuid_of(&body["details"]["deployment_id"]), *deployment);
    assert_eq!(deployments_of(fixture, server).await, recorded);
    assert_eq!(fleet_commands_of(fixture, server).await, 1);
    assert_requested_once(fixture, *deployment).await;

    // The caller recovers the outcome by reading the named deployment.
    let read = fixture
        .deployment(server, &json!({ "id": deployment }))
        .await;
    assert_eq!(read["state"], "requested", "{read}");
    assert_eq!(uuid_of(&read["artifact_id"]), artifact);
}

#[tokio::test]
async fn failure_injection_deployment_request_after_commit_is_recorded_once_and_the_retry_names_it()
{
    let suite = lock_suite().await;
    let fixture = MissionFixture::new(SUITE).await;
    let (mission, artifact, server) = deployable(&fixture, "Lost deployment answer").await;
    {
        let guard = suite.fail(Failpoint::DeploymentRequestAfterCommit);
        let (status, body) = fixture
            .request_deployment(&fixture.admin, server, mission, artifact, None)
            .await;
        assert_injected_failure(status, &body, Failpoint::DeploymentRequestAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    let retry = fixture
        .request_deployment(&fixture.admin, server, mission, artifact, None)
        .await;
    assert_lost_answer_recovers(&fixture, server, artifact, "web", retry).await;
}

#[tokio::test]
async fn failure_injection_relayed_deployment_request_after_commit_is_recorded_once_and_the_retry_names_it()
 {
    let suite = lock_suite().await;
    let fixture = MissionFixture::new(SUITE).await;
    let (mission, artifact, server) = deployable(&fixture, "Lost relayed answer").await;
    let runtime = machine(&fixture.credential(server, "mod_runtime").await);
    let relayed = json!({
        "mission_id": mission,
        "artifact_id": artifact,
        "requested_by_arma_id": format!("test-arma:{}", fixture.admin.id),
    });
    let route = "/api/v1/game-runtime/deployments";
    {
        let guard = suite.fail(Failpoint::DeploymentRequestAfterCommit);
        let (status, body) = fixture
            .call(Some(&runtime), "POST", route, Some(relayed.clone()))
            .await;
        assert_injected_failure(status, &body, Failpoint::DeploymentRequestAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    let retry = fixture
        .call(Some(&runtime), "POST", route, Some(relayed))
        .await;
    assert_lost_answer_recovers(&fixture, server, artifact, "game_runtime", retry).await;
}
