//! Route specs of the `missions_reviews` part: mission submission, the review thread and the
//! artifacts it decides, the administrators' approval queue, mission deployments requested on
//! the website or relayed from the game runtime, and the fleet scenario registry.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the fixture-dependent ownership, malformed and boundary probes, the one documented
//! divergence from a derived default, and a reason for every dimension that does not apply.

use api::missions::models::generated::mission_deployment::{
    DeployableMissionList, FleetScenario, FleetScenarioList, MissionDeployment,
    MissionDeploymentPage, RuntimeDeployment,
};
use api::missions::models::generated::mission_review::{
    ApprovalQueuePage, MissionArtifact, MissionReviewHistory, MissionRow, ReviewComment,
};
use serde_json::json;

use super::super::contracts::round_trip;
use super::super::spec::{
    Actor, Change, Contract, Dimension, Executor, Expect, Probe, Role, RouteSpec,
};

const MAKER: Actor = Actor::User(Role::MissionMaker);
const OTHER_MAKER: Actor = Actor::Peer(Role::MissionMaker);
const ADMIN: Actor = Actor::User(Role::Admin);
const RUNTIME: Actor = Actor::Machine(Executor::ModRuntime);
const OTHER_RUNTIME: Actor = Actor::MachineOtherServer(Executor::ModRuntime);

/// A fixed UUID no row carries: fixtures mint random ids, never this one.
const STRANGER_ID: &str = "00000000-0000-4000-8000-000000000000";

const DECISION: &str = "the approval queue is the administrators' shared worklist: any \
     administrator decides any submission";
const FLEET: &str = "administrators operate every server; a deployment has no owning account";
const REGISTRY: &str = "the fleet scenario registry is shared fleet configuration; no row is owned";
const RUNTIME_SCOPE: &str = "the machine credential names the target server; the body addresses \
     no other server";
const NO_INPUT: &str = "the route reads no path parameter, query or body";
const CREDENTIAL_SCOPE: &str = "no parameter, body or paging: the credential names the server";
const WHOLE_CATALOGUE: &str = "no parameter, body or paging: the list is every mission the \
     server can run";
const WHOLE_REGISTRY: &str = "no parameter, body or paging: the registry is one row per terrain";
const TERRAIN_TEXT: &str = "the terrain key is text and the route reads no body or query: every \
     key is a registered terrain or a 404";
const NOT_DEPLOYED_HERE: &str = "the runtime reads only artifacts a deployment of its own server \
     names; an unknown artifact is refused exactly like another server's, so the route never \
     reveals which artifacts exist";

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
fn author_or_administrator(spec: RouteSpec, live_fixture: &'static str) -> RouteSpec {
    spec.ownership(Probe::new("owner", MAKER))
        .ownership(
            Probe::new("another mission maker, mission under review", OTHER_MAKER)
                .expect_with(Expect::status(404).error_contains("mission not found")),
        )
        .ownership(
            Probe::new("another mission maker, live mission", OTHER_MAKER)
                .fixture(live_fixture)
                .expect_with(Expect::status(403).error_contains("not your mission")),
        )
        .ownership(Probe::new("administrator acts on any mission", ADMIN))
}

/// A mission-scoped read of one artifact: author or administrator, scoped to the mission.
fn artifact_read(key: &'static str, contract: Contract) -> RouteSpec {
    author_or_administrator(
        RouteSpec::authenticated(key)
            .ok(200, contract)
            .authorized_as(MAKER),
        "live-artifact",
    )
    .boundary(
        Probe::new("artifact of another mission", MAKER)
            .fixture("artifact-elsewhere")
            .expect_with(Expect::status(404).error_contains("artifact not found")),
    )
}

/// The limit fallback of every offset-paged list: out-of-range values answer the default.
fn paging(spec: RouteSpec) -> RouteSpec {
    spec.malformed(
        Probe::new("non-numeric limit", ADMIN)
            .query("limit=x")
            .expect(400),
    )
    .boundary(
        Probe::new("limit over the cap falls back to the default", ADMIN)
            .query("limit=101")
            .expect_with(Expect::success().json_at("/limit", json!(20))),
    )
    .boundary(
        Probe::new("limit at the cap", ADMIN)
            .query("limit=100")
            .expect_with(Expect::success().json_at("/limit", json!(100))),
    )
    .boundary(
        Probe::new("negative offset falls back to zero", ADMIN)
            .query("offset=-1")
            .expect_with(Expect::success().json_at("/offset", json!(0))),
    )
}

/// The shared malformed and boundary probes of the two review decisions.
fn decision(spec: RouteSpec, draft_fixture: &'static str) -> RouteSpec {
    spec.no_ownership(DECISION)
        .malformed(
            Probe::new("unknown field", ADMIN)
                .merge_body(json!({"reviewer": "someone else"}))
                .expect(400),
        )
        .malformed(
            Probe::new("artifact id is not a UUID", ADMIN)
                .merge_body(json!({"artifact_id": "not-a-uuid"}))
                .expect(400),
        )
        .boundary(
            Probe::new("stale artifact", ADMIN)
                .merge_body(json!({"artifact_id": STRANGER_ID}))
                .expect_with(refused(409, "REVIEWED_ARTIFACT_CHANGED")),
        )
        .boundary(
            Probe::new("draft mission is not pending", ADMIN)
                .fixture(draft_fixture)
                .expect_with(Expect::status(409).error_contains("not pending approval")),
        )
}

fn submission_and_review_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/missions/{id}/submit", Role::MissionMaker)
            .ok(200, review("MissionRow"))
            .decodes(round_trip::<MissionRow>)
            .ownership(Probe::new("owner submits", MAKER))
            .ownership(
                Probe::new("another mission maker", OTHER_MAKER)
                    .expect_with(Expect::status(403).error_contains("not your mission")),
            )
            .ownership(Probe::new("administrator submits for the author", ADMIN))
            .boundary(
                Probe::new("current version places no slots", MAKER)
                    .fixture("draft-without-slots")
                    .expect_with(refused(422, "NO_PLACED_SLOTS")),
            )
            .boundary(
                Probe::new("already under review", MAKER)
                    .fixture("mission-under-review")
                    .expect(409),
            )
            .boundary(
                Probe::new("live mission", MAKER)
                    .fixture("live-mission")
                    .expect(409),
            ),
        author_or_administrator(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/reviews")
                .ok(200, Contract::schema_root("mission-review.schema.json"))
                .decodes(round_trip::<MissionReviewHistory>)
                .authorized_as(MAKER),
            "live-mission",
        ),
        author_or_administrator(
            RouteSpec::role(
                "POST /api/v1/missions/{id}/review-comments",
                Role::MissionMaker,
            )
            .ok(201, review("ReviewComment"))
            .decodes(round_trip::<ReviewComment>)
            .request_contract(review("ReviewCommentRequest")),
            "live-comment",
        )
        .malformed(
            Probe::new("unknown field", MAKER)
                .merge_body(json!({"kind": "rejection"}))
                .expect(400),
        )
        .malformed(
            Probe::new("blank body", MAKER)
                .merge_body(json!({"body": "   "}))
                .expect(400),
        )
        .malformed(
            Probe::new("artifact of another mission", MAKER)
                .fixture("comment-foreign-artifact")
                .expect_with(Expect::status(400).error_contains("does not belong")),
        )
        .boundary(
            Probe::new("8000-byte comment", MAKER).merge_body(json!({"body": "c".repeat(8000)})),
        )
        .boundary(
            Probe::new("8001-byte comment", MAKER)
                .merge_body(json!({"body": "c".repeat(8001)}))
                .expect(400),
        ),
        artifact_read(
            "GET /api/v1/missions/{id}/artifacts/{artifact_id}",
            review("MissionArtifact"),
        )
        .decodes(round_trip::<MissionArtifact>),
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
                .ok(200, review("ApprovalQueuePage"))
                .decodes(round_trip::<ApprovalQueuePage>)
                .no_ownership(DECISION),
        ),
        decision(
            RouteSpec::role("POST /api/v1/approvals/{id}/approve", Role::Admin)
                .ok(200, review("MissionRow"))
                .decodes(round_trip::<MissionRow>)
                .request_contract(review("ApprovalDecision")),
            "approve-draft",
        )
        .malformed(
            Probe::new("missing artifact id", ADMIN)
                .change(Change::RemoveField("artifact_id"))
                .expect(400),
        )
        .boundary(
            Probe::new("approval with conditions", ADMIN)
                .merge_body(json!({"conditions": "Keep the medic slot"}))
                .expect_with(Expect::success().json_at("/status", json!("live"))),
        )
        .boundary(
            Probe::new("conditions over 8000 bytes", ADMIN)
                .merge_body(json!({"conditions": "c".repeat(8001)}))
                .expect(400),
        ),
        decision(
            RouteSpec::role("POST /api/v1/approvals/{id}/reject", Role::Admin)
                .ok(200, review("MissionRow"))
                .decodes(round_trip::<MissionRow>)
                .request_contract(review("RejectionDecision")),
            "reject-draft",
        )
        .malformed(
            Probe::new("missing reason", ADMIN)
                .change(Change::RemoveField("reason"))
                .expect(400),
        )
        .malformed(
            Probe::new("blank reason", ADMIN)
                .merge_body(json!({"reason": "  "}))
                .expect(400),
        )
        .boundary(
            Probe::new("8000-byte reason", ADMIN)
                .merge_body(json!({"reason": "r".repeat(8000)}))
                .expect_with(Expect::success().json_at("/status", json!("rejected"))),
        )
        .boundary(
            Probe::new("8001-byte reason", ADMIN)
                .merge_body(json!({"reason": "r".repeat(8001)}))
                .expect(400),
        ),
    ]
}

fn website_deployment_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("POST /api/v1/servers/{id}/deployments", Role::Admin)
            .ok(202, deployment("MissionDeployment"))
            .decodes(round_trip::<MissionDeployment>)
            .request_contract(deployment("DeploymentRequest"))
            .no_ownership(FLEET)
            .malformed(
                Probe::new("unknown field", ADMIN)
                    .merge_body(json!({"scenario_id": "{0123456789ABCDEF}Other.conf"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("mission id is not a UUID", ADMIN)
                    .merge_body(json!({"mission_id": "not-a-uuid"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("another deployment in flight", ADMIN)
                    .fixture("server-in-flight")
                    .expect_with(refused(409, "DEPLOYMENT_IN_PROGRESS")),
            )
            .boundary(
                Probe::new("deactivated server", ADMIN)
                    .fixture("inactive-server")
                    .expect_with(refused(409, "SERVER_INACTIVE")),
            )
            .boundary(
                Probe::new("artifact under review", ADMIN)
                    .fixture("unapproved-artifact")
                    .expect_with(refused(409, "ARTIFACT_NOT_APPROVED")),
            )
            .boundary(
                Probe::new("unknown mission", ADMIN)
                    .merge_body(json!({"mission_id": STRANGER_ID}))
                    .expect_with(Expect::status(404).error_contains("mission not found")),
            ),
        paging(
            RouteSpec::role("GET /api/v1/servers/{id}/deployments", Role::Admin)
                .ok(200, deployment("MissionDeploymentPage"))
                .decodes(round_trip::<MissionDeploymentPage>)
                .no_ownership(FLEET),
        )
        .boundary(
            Probe::new("one deployment per page", ADMIN)
                .query("limit=1")
                .expect_with(Expect::success().max_items("/items", 1)),
        ),
        RouteSpec::role(
            "GET /api/v1/servers/{id}/deployments/{deploymentId}",
            Role::Admin,
        )
        .ok(200, deployment("MissionDeployment"))
        .decodes(round_trip::<MissionDeployment>)
        .no_ownership(FLEET)
        .boundary(
            Probe::new("deployment of another server", ADMIN)
                .fixture("deployment-elsewhere")
                .expect_with(Expect::status(404).error_contains("deployment not found")),
        ),
        RouteSpec::role(
            "POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel",
            Role::Admin,
        )
        .ok(200, deployment("MissionDeployment"))
        .decodes(round_trip::<MissionDeployment>)
        .no_ownership(FLEET)
        .boundary(
            Probe::new("already cancelled", ADMIN)
                .fixture("cancelled-deployment")
                .expect_with(refused(409, "DEPLOYMENT_NOT_IN_FLIGHT")),
        )
        .boundary(
            Probe::new("deployment of another server", ADMIN)
                .fixture("deployment-elsewhere")
                .expect_with(Expect::status(404).error_contains("deployment not found")),
        ),
    ]
}

fn runtime_deployment_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine("GET /api/v1/game-runtime/deployment", Executor::ModRuntime)
            .ok(200, deployment("RuntimeDeployment"))
            .decodes(round_trip::<RuntimeDeployment>)
            .ownership(
                Probe::new("another server's runtime", OTHER_RUNTIME)
                    .expect_with(refused(404, "NO_DEPLOYMENT")),
            )
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, CREDENTIAL_SCOPE),
        RouteSpec::machine(
            "GET /api/v1/game-runtime/artifacts/{artifactId}",
            Executor::ModRuntime,
        )
        .ok(200, Contract::Binary("application/json"))
        .ownership(
            Probe::new("another server's runtime", OTHER_RUNTIME)
                .expect_with(refused(403, "ARTIFACT_NOT_DEPLOYED_HERE")),
        )
        .override_derived(
            "nonexistent artifactId",
            refused(403, "ARTIFACT_NOT_DEPLOYED_HERE"),
            NOT_DEPLOYED_HERE,
        )
        .boundary(
            Probe::new("artifact no deployment names", RUNTIME)
                .fixture("undeployed-artifact")
                .expect_with(refused(403, "ARTIFACT_NOT_DEPLOYED_HERE")),
        ),
        RouteSpec::machine("GET /api/v1/game-runtime/missions", Executor::ModRuntime)
            .ok(200, deployment("DeployableMissionList"))
            .decodes(round_trip::<DeployableMissionList>)
            .no_ownership("the list is scoped to the caller's server; it addresses no owned row")
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, WHOLE_CATALOGUE),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/deployments",
            Executor::ModRuntime,
        )
        .ok(202, deployment("MissionDeployment"))
        .decodes(round_trip::<MissionDeployment>)
        .request_contract(deployment("RelayedDeploymentRequest"))
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
        )
        .no_ownership(RUNTIME_SCOPE)
        .malformed(
            Probe::new("unknown field", RUNTIME)
                .merge_body(json!({"server_id": STRANGER_ID}))
                .expect(400),
        )
        .malformed(
            Probe::new("missing in-game identity", RUNTIME)
                .change(Change::RemoveField("requested_by_arma_id"))
                .expect(400),
        )
        .boundary(
            Probe::new("another deployment in flight", RUNTIME)
                .fixture("in-game-in-flight")
                .expect_with(refused(409, "DEPLOYMENT_IN_PROGRESS")),
        )
        .boundary(
            Probe::new("artifact under review", RUNTIME)
                .fixture("in-game-unapproved")
                .expect_with(refused(409, "ARTIFACT_NOT_APPROVED")),
        ),
    ]
}

fn fleet_scenario_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/fleet/scenarios", Role::Admin)
            .ok(200, deployment("FleetScenarioList"))
            .decodes(round_trip::<FleetScenarioList>)
            .no_ownership(REGISTRY)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, WHOLE_REGISTRY),
        RouteSpec::role("PUT /api/v1/fleet/scenarios/{terrainKey}", Role::Admin)
            .text_param("terrainKey")
            .ok(200, deployment("FleetScenario"))
            .decodes(round_trip::<FleetScenario>)
            .request_contract(deployment("FleetScenarioUpdate"))
            .no_ownership(REGISTRY)
            .malformed(
                Probe::new("uppercase terrain key", ADMIN)
                    .param("terrainKey", "Everon")
                    .expect(400),
            )
            .malformed(
                Probe::new("scenario id without a resource GUID", ADMIN)
                    .merge_body(json!({"scenario_id": "Missions/RouteAcceptance.conf"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown field", ADMIN)
                    .merge_body(json!({"terrain_key": "everon"}))
                    .expect(400),
            )
            .boundary(Probe::new("64-byte terrain key", ADMIN).param("terrainKey", "k".repeat(64)))
            .boundary(
                Probe::new("65-byte terrain key", ADMIN)
                    .param("terrainKey", "k".repeat(65))
                    .expect(400),
            )
            .boundary(
                Probe::new("128-byte display name", ADMIN)
                    .merge_body(json!({"display_name": "d".repeat(128)})),
            )
            .boundary(
                Probe::new("129-byte display name", ADMIN)
                    .merge_body(json!({"display_name": "d".repeat(129)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("blank display name", ADMIN)
                    .merge_body(json!({"display_name": "  "}))
                    .expect(400),
            ),
        RouteSpec::role("DELETE /api/v1/fleet/scenarios/{terrainKey}", Role::Admin)
            .text_param("terrainKey")
            .ok(204, Contract::NoBody)
            .no_ownership(REGISTRY)
            .not_applicable(Dimension::Malformed, TERRAIN_TEXT)
            .boundary(
                Probe::new("unregistered terrain", ADMIN)
                    .param("terrainKey", "route_acceptance_never_registered")
                    .expect_with(Expect::status(404).error_contains("no scenario")),
            ),
    ]
}

/// Every spec of the `missions_reviews` part.
pub fn specs() -> Vec<RouteSpec> {
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
