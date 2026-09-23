//! The mission review, artifact, approval and deployment wire as the backend actually serves it
//! satisfies `mission-review.schema.json` and `mission-deployment.schema.json`, and the types
//! generated from those schemas decode it; the request bodies the routes accept are the ones the
//! schemas describe.

mod common;
mod contract_support;
mod mission_artifact_support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use contract_support::{assert_decodes, assert_invalid, assert_valid};
use mission_artifact_support::{MissionFixture, machine};
use website_api::missions::models::generated::{mission_deployment, mission_review};

const SUITE: &str = "mission_review_contract";
const SCHEMA: &str = "mission-review.schema.json";
const DEPLOYMENT: &str = "mission-deployment.schema.json";

fn assert_contract<T: serde::de::DeserializeOwned>(definition: Option<&str>, value: &Value) {
    assert_valid(SCHEMA, definition, value);
    assert_decodes::<T>(definition.unwrap_or("MissionReviewHistory"), value);
}

#[tokio::test]
async fn mission_review_wire_satisfies_the_published_contract() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Contract review").await;

    let (status, submitted) = f.submit_as(&f.author, mission).await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_contract::<mission_review::MissionRow>(Some("MissionRow"), &submitted);

    let (status, queue) = f
        .call(Some(&f.admin), "GET", "/api/v1/approvals?limit=100", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    assert_contract::<mission_review::ApprovalQueuePage>(Some("ApprovalQueuePage"), &queue);
    let artifact = f.pending_artifact(mission).await.unwrap();

    let (status, provenance) = f.artifact(&f.author, mission, artifact).await;
    assert_eq!(status, StatusCode::OK, "{provenance}");
    assert_contract::<mission_review::MissionArtifact>(Some("MissionArtifact"), &provenance);

    let (status, workspace) = f
        .call(
            Some(&f.admin),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact}/workspace"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{workspace}");
    assert_contract::<mission_review::ReviewWorkspace>(Some("ReviewWorkspace"), &workspace);

    let request = json!({ "body": "Radios checked", "artifact_id": artifact });
    assert_contract::<mission_review::ReviewCommentRequest>(Some("ReviewCommentRequest"), &request);
    let (status, comment) = f
        .call(
            Some(&f.admin),
            "POST",
            &format!("/api/v1/missions/{mission}/review-comments"),
            Some(request),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{comment}");
    assert_contract::<mission_review::ReviewComment>(Some("ReviewComment"), &comment);

    let (_, pending) = f.reviews(&f.author, mission).await;
    assert_contract::<mission_review::MissionReviewHistory>(None, &pending);

    let decision = json!({ "artifact_id": artifact, "conditions": "Night rotation only" });
    assert_contract::<mission_review::ApprovalDecision>(Some("ApprovalDecision"), &decision);
    let (status, decided) = f
        .call(
            Some(&f.admin),
            "POST",
            &format!("/api/v1/approvals/{mission}/approve"),
            Some(decision),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_contract::<mission_review::MissionRow>(Some("MissionRow"), &decided);
    assert!(decided.get("approved_artifact_id").is_some(), "{decided}");

    let (_, history) = f.reviews(&f.author, mission).await;
    assert_contract::<mission_review::MissionReviewHistory>(None, &history);
    assert!(
        history["reviews"][0].get("decided_at").is_some(),
        "{history}"
    );

    // A rejection names its artifact and reason, and answers the rejected row.
    let (other, _) = f.compilable_mission("Contract rejection").await;
    let rejected_artifact = f.submit(other).await;
    let rejection = json!({ "artifact_id": rejected_artifact, "reason": "Move the spawn" });
    assert_contract::<mission_review::RejectionDecision>(Some("RejectionDecision"), &rejection);
    let (status, rejected) = f
        .call(
            Some(&f.admin),
            "POST",
            &format!("/api/v1/approvals/{other}/reject"),
            Some(rejection),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{rejected}");
    assert_contract::<mission_review::MissionRow>(Some("MissionRow"), &rejected);
}

#[test]
fn mission_review_contract_refuses_what_the_routes_refuse() {
    let artifact = "7c9e6679-7425-40de-944b-e07fc1f90ae7";
    for (definition, value) in [
        ("ApprovalDecision", json!({})),
        ("ApprovalDecision", json!({ "artifact_id": "not-a-uuid" })),
        (
            "ApprovalDecision",
            json!({ "artifact_id": artifact, "extra": true }),
        ),
        ("RejectionDecision", json!({ "artifact_id": artifact })),
        (
            "RejectionDecision",
            json!({ "artifact_id": artifact, "reason": "   " }),
        ),
        (
            "RejectionDecision",
            json!({ "artifact_id": artifact, "reason": "x".repeat(8001) }),
        ),
        ("ReviewCommentRequest", json!({ "body": "" })),
        ("ReviewCommentRequest", json!({ "body": "\t \n" })),
        (
            "ReviewCommentRequest",
            json!({ "body": "ok", "kind": "rejection" }),
        ),
    ] {
        assert_invalid(SCHEMA, Some(definition), &value);
    }
    let review = json!({
        "id": artifact, "mission_id": artifact, "artifact_id": artifact,
        "artifact_digest": "a".repeat(64), "mission_version_id": artifact, "semver": "0.2.0",
        "submitted_by": "author", "submitted_at": "2026-09-23T10:00:00Z", "state": "pending"
    });
    assert_valid(SCHEMA, Some("MissionReview"), &review);
    let mut unknown_state = review.clone();
    unknown_state["state"] = json!("withdrawn");
    assert_invalid(SCHEMA, Some("MissionReview"), &unknown_state);
    let mut short_digest = review;
    short_digest["artifact_digest"] = json!("abc");
    assert_invalid(SCHEMA, Some("MissionReview"), &short_digest);
}

fn assert_deployment_contract<T: serde::de::DeserializeOwned>(definition: &str, value: &Value) {
    assert_valid(DEPLOYMENT, Some(definition), value);
    assert_decodes::<T>(definition, value);
}

#[tokio::test]
async fn mission_deployment_wire_satisfies_the_published_contract() {
    let f = MissionFixture::new(SUITE).await;
    let update = json!({ "scenario_id": "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf", "display_name": "Everon" });
    assert_deployment_contract::<mission_deployment::FleetScenarioUpdate>(
        "FleetScenarioUpdate",
        &update,
    );
    let (status, scenario) = f
        .call(
            Some(&f.admin),
            "PUT",
            "/api/v1/fleet/scenarios/everon",
            Some(update),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{scenario}");
    assert_deployment_contract::<mission_deployment::FleetScenario>("FleetScenario", &scenario);
    let (_, scenarios) = f
        .call(Some(&f.admin), "GET", "/api/v1/fleet/scenarios", None)
        .await;
    assert_deployment_contract::<mission_deployment::FleetScenarioList>(
        "FleetScenarioList",
        &scenarios,
    );

    let (mission, artifact) = f.approved_mission("Contract deployment").await;
    let modpack: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT modpack_id FROM mission_artifacts WHERE id = $1")
            .bind(artifact)
            .fetch_one(f.pool())
            .await
            .unwrap();
    let server = f.register_server("Contract host", modpack).await;
    let runtime = f.credential(server, "mod_runtime").await;
    let (_, event_mission) = f.event_on(server, mission).await;
    let request = json!({ "mission_id": mission, "artifact_id": artifact, "event_mission_id": event_mission });
    assert_deployment_contract::<mission_deployment::DeploymentRequest>(
        "DeploymentRequest",
        &request,
    );
    let (status, deployment) = f
        .call(
            Some(&f.admin),
            "POST",
            &format!("/api/v1/servers/{server}/deployments"),
            Some(request),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{deployment}");
    assert_valid(DEPLOYMENT, None, &deployment);
    assert_decodes::<mission_deployment::MissionDeployment>("deployment", &deployment);

    let (status, running) = f
        .call(
            Some(&machine(&runtime)),
            "GET",
            "/api/v1/game-runtime/deployment",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{running}");
    assert_deployment_contract::<mission_deployment::RuntimeDeployment>(
        "RuntimeDeployment",
        &running,
    );
    let (_, offered) = f
        .call(
            Some(&machine(&runtime)),
            "GET",
            "/api/v1/game-runtime/missions",
            None,
        )
        .await;
    assert_deployment_contract::<mission_deployment::DeployableMissionList>(
        "DeployableMissionList",
        &offered,
    );

    let sha256: String = running["artifact_sha256"].as_str().unwrap().to_owned();
    let (status, _) = f.start_session(&runtime, Some((artifact, &sha256))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, page) = f
        .call(
            Some(&f.admin),
            "GET",
            &format!("/api/v1/servers/{server}/deployments"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_deployment_contract::<mission_deployment::MissionDeploymentPage>(
        "MissionDeploymentPage",
        &page,
    );
    assert_eq!(page["items"][0]["state"], "confirmed", "{page}");

    let relayed = json!({ "mission_id": mission, "artifact_id": artifact, "requested_by_arma_id": "test-arma:someone" });
    assert_deployment_contract::<mission_deployment::RelayedDeploymentRequest>(
        "RelayedDeploymentRequest",
        &relayed,
    );
    for (definition, value) in [
        ("DeploymentRequest", json!({ "mission_id": mission })),
        (
            "DeploymentRequest",
            json!({ "mission_id": mission, "artifact_id": artifact, "extra": 1 }),
        ),
        (
            "RelayedDeploymentRequest",
            json!({ "mission_id": mission, "artifact_id": artifact }),
        ),
        (
            "FleetScenarioUpdate",
            json!({ "scenario_id": "Missions/No_Guid.conf", "display_name": "x" }),
        ),
        (
            "FleetScenarioUpdate",
            json!({ "scenario_id": "{69A85365FC09E2CA}Missions/A.conf", "display_name": "  " }),
        ),
    ] {
        assert_invalid(DEPLOYMENT, Some(definition), &value);
    }
}
