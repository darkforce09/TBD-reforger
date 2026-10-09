//! Route specs of the `missions_reviews` part: mission submission, the review thread and the
//! artifacts it decides, the administrators' approval queue, mission deployments requested on
//! the website or relayed from the game runtime, and the fleet scenario registry.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use serde_json::json;

use super::super::spec::{Actor, Contract, Executor, Expect, Probe, Role, RouteSpec};

const MAKER: Actor = Actor::User(Role::MissionMaker);
const RUNTIME: Actor = Actor::Machine(Executor::ModRuntime);

fn review(definition: &'static str) -> Contract {
    Contract::schema("mission-review.schema.json", definition)
}

fn deployment(definition: &'static str) -> Contract {
    Contract::schema("mission-deployment.schema.json", definition)
}

fn refused(status: u16, code: &'static str) -> Expect {
    Expect::status(status).details_code(code)
}

/// The author-or-administrator rule of the mission review routes: a mission the caller cannot
/// see answers 404 like a missing one, a visible (live) mission the caller does not own 403.
fn author_or_administrator(spec: RouteSpec, _live_fixture: &'static str) -> RouteSpec {
    spec
}

/// A mission-scoped read of one artifact: author or administrator, scoped to the mission.
fn artifact_read(key: &'static str, contract: Contract) -> RouteSpec {
    author_or_administrator(
        RouteSpec::authenticated(key)
            .ok(200, contract)
            .authorized_as(MAKER),
        "live-artifact",
    )
}

/// The limit fallback of every offset-paged list: out-of-range values answer the default.
fn paging(spec: RouteSpec) -> RouteSpec {
    spec
}

/// The shared malformed and boundary probes of the two review decisions.
fn decision(spec: RouteSpec, _draft_fixture: &'static str) -> RouteSpec {
    spec
}

fn submission_and_review_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/missions/{id}/submit", Role::MissionMaker)
            .ok(200, review("MissionRow")),
        author_or_administrator(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/reviews")
                .ok(200, Contract::schema_root("mission-review.schema.json"))
                .authorized_as(MAKER),
            "live-mission",
        ),
        author_or_administrator(
            RouteSpec::role(
                "POST /api/v1/missions/{id}/review-comments",
                Role::MissionMaker,
            )
            .ok(201, review("ReviewComment")),
            "live-comment",
        ),
        artifact_read(
            "GET /api/v1/missions/{id}/artifacts/{artifact_id}",
            review("MissionArtifact"),
        ),
        artifact_read(
            "GET /api/v1/missions/{id}/artifacts/{artifact_id}/document",
            Contract::Binary("application/json"),
        ),
    ]
}

fn approval_specs() -> Vec<RouteSpec> {
    vec![
        paging(
            RouteSpec::role("GET /api/v1/approvals", Role::Admin)
                .ok(200, review("ApprovalQueuePage")),
        ),
        decision(
            RouteSpec::role("POST /api/v1/approvals/{id}/approve", Role::Admin)
                .ok(200, review("MissionRow")),
            "approve-draft",
        ),
        decision(
            RouteSpec::role("POST /api/v1/approvals/{id}/reject", Role::Admin)
                .ok(200, review("MissionRow")),
            "reject-draft",
        ),
    ]
}

fn website_deployment_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/servers/{id}/deployments", Role::Admin)
            .ok(202, deployment("MissionDeployment")),
        paging(
            RouteSpec::role("GET /api/v1/servers/{id}/deployments", Role::Admin)
                .ok(200, deployment("MissionDeploymentPage")),
        ),
        RouteSpec::role(
            "GET /api/v1/servers/{id}/deployments/{deploymentId}",
            Role::Admin,
        )
        .ok(200, deployment("MissionDeployment")),
        RouteSpec::role(
            "POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel",
            Role::Admin,
        )
        .ok(200, deployment("MissionDeployment")),
    ]
}

fn runtime_deployment_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("GET /api/v1/game-runtime/deployment", Executor::ModRuntime)
            .ok(200, deployment("RuntimeDeployment")),
        RouteSpec::machine(
            "GET /api/v1/game-runtime/artifacts/{artifactId}",
            Executor::ModRuntime,
        )
        .ok(200, Contract::Binary("application/json")),
        RouteSpec::machine("GET /api/v1/game-runtime/missions", Executor::ModRuntime)
            .ok(200, deployment("DeployableMissionList")),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/deployments",
            Executor::ModRuntime,
        )
        .ok(202, deployment("MissionDeployment"))
        .unauthorized(
            Probe::new("unlinked in-game identity", RUNTIME)
                .merge_body(json!({"requested_by_arma_id": "route-acceptance-unlinked"}))
                .expect_with(refused(403, "IDENTITY_NOT_LINKED")),
        )
        .unauthorized(
            Probe::new("enlisted in-game identity", RUNTIME)
                .fixture("in-game-enlisted")
                .expect_with(refused(403, "NOT_AN_ADMINISTRATOR")),
        )
        .unauthorized(
            Probe::new("banned administrator's in-game identity", RUNTIME)
                .fixture("in-game-banned")
                .expect_with(refused(403, "NOT_AN_ADMINISTRATOR")),
        ),
    ]
}

fn fleet_scenario_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/fleet/scenarios", Role::Admin)
            .ok(200, deployment("FleetScenarioList")),
        RouteSpec::role("PUT /api/v1/fleet/scenarios/{terrainKey}", Role::Admin)
            .ok(200, deployment("FleetScenario")),
        RouteSpec::role("DELETE /api/v1/fleet/scenarios/{terrainKey}", Role::Admin)
            .ok(204, Contract::NoBody),
    ]
}

/// Every spec of the `missions_reviews` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [
        submission_and_review_specs(),
        approval_specs(),
        website_deployment_specs(),
        runtime_deployment_specs(),
        fleet_scenario_specs(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
