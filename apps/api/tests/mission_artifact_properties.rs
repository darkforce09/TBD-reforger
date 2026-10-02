//! Generated mission lifecycles show that approval and deployment name one immutable artifact.
//!
//! **Role:** the property `approval_and_deployment_share_immutable_artifact`: generated
//! save/submit/approve/reject/reopen/deployment-request/runtime-report sequences run through the
//! production routes, and an oracle of the mission lifecycle predicts every answer.
//!
//! **Position:** a `api` integration binary over its per-binary test database. Requests go
//! through `http_router::router` with the accounts of `MissionFixture`
//! (`tests/mission_artifact_support`); the invariants read the persisted mission, review,
//! artifact, deployment, fleet command and runtime session rows.
//!
//! **Signals & state:** one fixture, one Tokio runtime and one synthetic peer counter for the whole
//! binary; each generated case owns a fresh mission, server and runtime credential, so no case
//! observes another's lifecycle.
//!
//! **Invariants:** every accepted deployment names the artifact the mission's latest approval
//! decided, with that artifact's document SHA-256, in its response, its row and its fleet command;
//! an artifact row never changes once written and always records the SHA-256 of its document; a
//! decision naming another artifact than the one under review changes nothing, so a stale approval
//! never deploys a newer artifact; a deployment is confirmed only by a runtime report of its exact
//! artifact and bytes.

mod common;
mod mission_artifact_support;

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use proptest::collection::vec;
use proptest::prelude::*;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use mission_artifact_support::{
    MissionFixture, payload_with_role, refusal_code, sha256_hex, uuid_of,
};

const SUITE: &str = "mission_artifact_properties";
const EVERON_SCENARIO: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

/// The artifact an operation names, resolved against the oracle when the operation runs; an
/// artifact the oracle does not hold resolves to an id no artifact has.
#[derive(Debug, Clone, Copy)]
enum Target {
    UnderReview,
    Approved,
    Deployed,
    Oldest,
    Newest,
    Unknown,
}

/// One generated step of a mission's lifecycle.
#[derive(Debug, Clone, Copy)]
enum Operation {
    /// The author saves a new version with its own role, so it compiles to other bytes.
    SaveVersion,
    /// The author submits the current version for review.
    Submit,
    Approve {
        artifact: Target,
        with_conditions: bool,
    },
    Reject(Target),
    /// The author archives the mission and restores it to draft, making a live mission
    /// resubmittable.
    Reopen,
    /// An administrator requests a deployment of the artifact on the case's server.
    RequestDeployment(Target),
    /// The server's runtime starts a session reporting the artifact it loaded, with the artifact's
    /// own document SHA-256 or bytes no artifact has.
    ReportLoaded {
        artifact: Target,
        exact_bytes: bool,
    },
}

/// Decisions mostly name the artifact under review; the rest name a stale or unknown artifact.
fn decision_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        6 => Just(Target::UnderReview),
        1 => Just(Target::Approved),
        1 => Just(Target::Oldest),
        1 => Just(Target::Newest),
        1 => Just(Target::Unknown),
    ]
}

/// Deployment requests mostly name the approved artifact; the rest name another artifact of the
/// mission or an unknown one.
fn deployment_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        5 => Just(Target::Approved),
        1 => Just(Target::UnderReview),
        1 => Just(Target::Deployed),
        2 => Just(Target::Oldest),
        2 => Just(Target::Newest),
        1 => Just(Target::Unknown),
    ]
}

/// Runtime reports mostly name the deployed artifact; the rest name another or an unknown one.
fn report_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        5 => Just(Target::Deployed),
        1 => Just(Target::Approved),
        1 => Just(Target::Oldest),
        1 => Just(Target::Newest),
        1 => Just(Target::Unknown),
    ]
}

fn decision() -> impl Strategy<Value = Operation> {
    prop_oneof![
        13 => (decision_target(), any::<bool>()).prop_map(|(artifact, with_conditions)| {
            Operation::Approve { artifact, with_conditions }
        }),
        7 => decision_target().prop_map(Operation::Reject),
    ]
}

fn deployment() -> impl Strategy<Value = Operation> {
    deployment_target().prop_map(Operation::RequestDeployment)
}

fn report() -> impl Strategy<Value = Operation> {
    (report_target(), any::<bool>()).prop_map(|(artifact, exact_bytes)| Operation::ReportLoaded {
        artifact,
        exact_bytes,
    })
}

/// Any operation, for the steps a round interleaves between a submission and its decision.
fn any_operation() -> impl Strategy<Value = Operation> {
    prop_oneof![
        3 => Just(Operation::SaveVersion),
        4 => Just(Operation::Submit),
        6 => decision(),
        2 => Just(Operation::Reopen),
        4 => deployment(),
        3 => report(),
    ]
}

/// One lifecycle round: optionally a new version and a reopening, a submission, up to two
/// operations of any kind, a decision, a deployment request, a runtime report, and optionally a
/// second deployment request. The optional and generated targets let every step also be refused.
fn round() -> impl Strategy<Value = Vec<Operation>> {
    (
        proptest::option::weighted(0.6, Just(Operation::SaveVersion)),
        proptest::option::weighted(0.4, Just(Operation::Reopen)),
        vec(any_operation(), 0..3),
        decision(),
        deployment(),
        report(),
        proptest::option::weighted(0.5, deployment()),
    )
        .prop_map(
            |(save, reopen, interleaved, decision, deployment, report, second)| {
                let mut operations: Vec<Operation> = save.into_iter().chain(reopen).collect();
                operations.push(Operation::Submit);
                operations.extend(interleaved);
                operations.extend([decision, deployment, report]);
                operations.extend(second);
                operations
            },
        )
}

/// One to four rounds of a mission's lifecycle, as one operation sequence.
fn lifecycle() -> impl Strategy<Value = Vec<Operation>> {
    vec(round(), 1..5).prop_map(|rounds| rounds.into_iter().flatten().collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Draft,
    PendingApproval,
    Live,
    Rejected,
}

impl Status {
    fn wire(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingApproval => "pending_approval",
            Self::Live => "live",
            Self::Rejected => "rejected",
        }
    }
}

enum Decision {
    Approve { with_conditions: bool },
    Reject,
}

/// What the mission, its reviews and the server's deployments must be after each step.
struct Oracle {
    status: Status,
    current_version: Option<Uuid>,
    saved_versions: u32,
    artifact_of_version: BTreeMap<Uuid, Uuid>,
    /// Every artifact of the mission, in the order submissions compiled them.
    artifacts: Vec<Uuid>,
    /// The artifact of the pending review; it stays pending while a reopened mission is a draft.
    under_review: Option<Uuid>,
    approved: Option<Uuid>,
    /// The deployment in flight and its artifact.
    in_flight: Option<(Uuid, Uuid)>,
    /// The artifact of the latest accepted deployment.
    deployed: Option<Uuid>,
    /// Every accepted deployment: its artifact and its state.
    deployments: BTreeMap<Uuid, (Uuid, &'static str)>,
}

impl Oracle {
    fn resolve(&self, target: Target) -> Uuid {
        let named = match target {
            Target::UnderReview => self.under_review,
            Target::Approved => self.approved,
            Target::Deployed => self.deployed,
            Target::Oldest => self.artifacts.first().copied(),
            Target::Newest => self.artifacts.last().copied(),
            Target::Unknown => None,
        };
        named.unwrap_or_else(Uuid::new_v4)
    }
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// A fresh synthetic client address, so the per-address rate limit never throttles a case.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

/// One request through the production router with `bearer`, from a fresh peer.
async fn call(
    f: &MissionFixture,
    bearer: &str,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let mut request = request
        .body(body.map_or(Body::empty(), |body| Body::from(body.to_string())))
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = f.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn assert_refused(step: &str, answer: &(StatusCode, Value), status: StatusCode, code: &str) {
    let (actual, body) = answer;
    assert_eq!(
        (*actual, refusal_code(body)),
        (status, code),
        "{step}: {body}"
    );
    assert!(
        body["error"].is_string(),
        "{step}: a refusal carries the error envelope: {body}"
    );
}

#[derive(sqlx::FromRow)]
struct DeploymentRow {
    id: Uuid,
    mission_id: Uuid,
    artifact_id: Uuid,
    state: String,
    document_sha256: String,
    command_artifact: Option<String>,
    command_sha256: Option<String>,
    loaded_artifact_id: Option<Uuid>,
    loaded_artifact_sha256: Option<String>,
    approval_in_force: Option<Uuid>,
}

/// The server's deployments with their artifact, fleet command arguments, confirming session and
/// the artifact of the mission's latest approval decided before each request.
const DEPLOYMENT_ROWS: &str =
    "SELECT d.id, d.mission_id, d.artifact_id, d.state, a.document_sha256,
        c.arguments->>'artifact_id' AS command_artifact,
        c.arguments->>'artifact_sha256' AS command_sha256,
        s.loaded_artifact_id, s.loaded_artifact_sha256,
        (SELECT r.artifact_id FROM mission_reviews r
         WHERE r.mission_id = d.mission_id AND r.state IN ('approved', 'approved_with_conditions')
           AND r.decided_at <= d.requested_at
         ORDER BY r.decided_at DESC, r.id DESC LIMIT 1) AS approval_in_force
    FROM mission_deployments d
    JOIN mission_artifacts a ON a.id = d.artifact_id
    JOIN fleet_commands c ON c.id = d.fleet_command_id
    LEFT JOIN server_runtime_sessions s ON s.id = d.confirmed_runtime_session_id
    WHERE d.server_id = $1";

/// How often the generated cases reached the outcomes the property is about; the binary fails
/// when a run leaves one of them unexercised.
#[derive(Debug, Default)]
struct Reached {
    deployments_accepted: u32,
    deployments_confirmed: u32,
    deployments_failed: u32,
    /// A live mission refused a deployment of one of its own artifacts other than the approved one.
    unapproved_artifacts_of_live_missions_refused: u32,
    /// A decision naming one of the mission's artifacts other than the one under review.
    stale_decisions_refused: u32,
    /// An approval replaced an earlier approval of another artifact.
    approvals_replaced: u32,
}

impl Reached {
    fn absorb(&mut self, case: &Self) {
        self.deployments_accepted += case.deployments_accepted;
        self.deployments_confirmed += case.deployments_confirmed;
        self.deployments_failed += case.deployments_failed;
        self.unapproved_artifacts_of_live_missions_refused +=
            case.unapproved_artifacts_of_live_missions_refused;
        self.stale_decisions_refused += case.stale_decisions_refused;
        self.approvals_replaced += case.approvals_replaced;
    }

    fn assert_every_outcome_reached(&self) {
        let counts = [
            self.deployments_accepted,
            self.deployments_confirmed,
            self.deployments_failed,
            self.unapproved_artifacts_of_live_missions_refused,
            self.stale_decisions_refused,
            self.approvals_replaced,
        ];
        assert!(
            counts.iter().all(|count| *count > 0),
            "the generated cases leave an outcome unexercised: {self:?}"
        );
    }
}

/// One generated case: its mission, its server and runtime credential, the oracle, every
/// artifact row as first read, `(row fingerprint, document SHA-256)`, and the outcomes it reached.
struct Case<'a> {
    f: &'a MissionFixture,
    mission: Uuid,
    server: Uuid,
    runtime_secret: String,
    oracle: Oracle,
    artifact_rows: BTreeMap<Uuid, (String, String)>,
    reached: Reached,
}

impl<'a> Case<'a> {
    /// A draft mission with one saved version, and a server requiring `modpack` with a runtime
    /// credential.
    async fn open(f: &'a MissionFixture, modpack: Option<Uuid>) -> Self {
        let (status, created) = call(
            f,
            &f.author.token,
            "POST",
            "/api/v1/missions",
            Some(json!({
                "title": "Artifact property", "terrain": "everon", "game_mode": "pve_coop",
                "max_players": 16
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        let server = f
            .register_server(
                &format!("Artifact property host {}", Uuid::new_v4()),
                modpack,
            )
            .await;
        let (status, credential) = call(
            f,
            &f.admin.token,
            "POST",
            &format!("/api/v1/servers/{server}/credentials"),
            Some(json!({ "executor_kind": "mod_runtime", "label": "artifact property runtime" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{credential}");
        let mut case = Self {
            f,
            mission: uuid_of(&created["id"]),
            server,
            runtime_secret: credential["secret"].as_str().unwrap().to_owned(),
            oracle: Oracle {
                status: Status::Draft,
                current_version: None,
                saved_versions: 0,
                artifact_of_version: BTreeMap::new(),
                artifacts: Vec::new(),
                under_review: None,
                approved: None,
                in_flight: None,
                deployed: None,
                deployments: BTreeMap::new(),
            },
            artifact_rows: BTreeMap::new(),
            reached: Reached::default(),
        };
        case.save_version("setup").await;
        case
    }
}

impl Case<'_> {
    async fn apply(&mut self, step: &str, operation: Operation) {
        match operation {
            Operation::SaveVersion => self.save_version(step).await,
            Operation::Submit => self.submit(step).await,
            Operation::Approve {
                artifact,
                with_conditions,
            } => {
                self.decide(step, artifact, Decision::Approve { with_conditions })
                    .await
            }
            Operation::Reject(artifact) => self.decide(step, artifact, Decision::Reject).await,
            Operation::Reopen => self.reopen(step).await,
            Operation::RequestDeployment(artifact) => self.request_deployment(step, artifact).await,
            Operation::ReportLoaded {
                artifact,
                exact_bytes,
            } => self.report_loaded(step, artifact, exact_bytes).await,
        }
    }

    /// Save version `1.<n>.0`; the `0.x` range belongs to the version a new mission starts with.
    async fn save_version(&mut self, step: &str) {
        self.oracle.saved_versions += 1;
        let n = self.oracle.saved_versions;
        let payload: Value = serde_json::from_str(&payload_with_role(&format!("R{n}"))).unwrap();
        let (status, body) = call(
            self.f,
            &self.f.author.token,
            "POST",
            &format!("/api/v1/missions/{}/versions", self.mission),
            Some(json!({ "semver": format!("1.{n}.0"), "payload": payload })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{step}: {body}");
        self.oracle.current_version = Some(uuid_of(&body["id"]));
    }

    async fn submit(&mut self, step: &str) {
        let answer = call(
            self.f,
            &self.f.author.token,
            "POST",
            &format!("/api/v1/missions/{}/submit", self.mission),
            None,
        )
        .await;
        if !matches!(self.oracle.status, Status::Draft | Status::Rejected) {
            assert_refused(step, &answer, StatusCode::CONFLICT, "");
            return;
        }
        let (status, body) = answer;
        assert_eq!(status, StatusCode::OK, "{step}: {body}");
        assert_eq!(body["status"], "pending_approval", "{step}: {body}");
        let (artifact, version): (Uuid, Uuid) = sqlx::query_as(
            "SELECT r.artifact_id, a.mission_version_id FROM mission_reviews r
             JOIN mission_artifacts a ON a.id = r.artifact_id
             WHERE r.mission_id = $1 AND r.state = 'pending'",
        )
        .bind(self.mission)
        .fetch_one(self.f.pool())
        .await
        .unwrap();
        let current = self.oracle.current_version.unwrap();
        assert_eq!(
            version, current,
            "{step}: the review decides the current version's artifact"
        );
        match self.oracle.artifact_of_version.get(&current) {
            Some(compiled) => assert_eq!(
                artifact, *compiled,
                "{step}: recompiling a version answers its existing artifact"
            ),
            None => {
                assert!(
                    !self.oracle.artifacts.contains(&artifact),
                    "{step}: a new version compiles to a new artifact"
                );
                self.oracle.artifact_of_version.insert(current, artifact);
                self.oracle.artifacts.push(artifact);
            }
        }
        self.oracle.status = Status::PendingApproval;
        self.oracle.under_review = Some(artifact);
    }

    async fn decide(&mut self, step: &str, target: Target, decision: Decision) {
        let artifact = self.oracle.resolve(target);
        let (route, request) = match decision {
            Decision::Approve { with_conditions } => {
                let mut request = json!({ "artifact_id": artifact });
                if with_conditions {
                    request["conditions"] = json!("Generated approval conditions");
                }
                ("approve", request)
            }
            Decision::Reject => (
                "reject",
                json!({ "artifact_id": artifact, "reason": "Generated rejection" }),
            ),
        };
        let answer = call(
            self.f,
            &self.f.admin.token,
            "POST",
            &format!("/api/v1/approvals/{}/{route}", self.mission),
            Some(request),
        )
        .await;
        if self.oracle.status != Status::PendingApproval {
            assert_refused(step, &answer, StatusCode::CONFLICT, "");
            return;
        }
        let under_review = self
            .oracle
            .under_review
            .expect("a mission pending approval has an artifact under review");
        if artifact != under_review {
            assert_refused(
                step,
                &answer,
                StatusCode::CONFLICT,
                "REVIEWED_ARTIFACT_CHANGED",
            );
            assert_eq!(
                uuid_of(&answer.1["details"]["artifact_id"]),
                under_review,
                "{step}: the refusal names the artifact under review"
            );
            if self.oracle.artifacts.contains(&artifact) {
                self.reached.stale_decisions_refused += 1;
            }
            return;
        }
        let (status, body) = answer;
        assert_eq!(status, StatusCode::OK, "{step}: {body}");
        self.oracle.under_review = None;
        match decision {
            Decision::Approve { .. } => {
                self.oracle.status = Status::Live;
                if self
                    .oracle
                    .approved
                    .is_some_and(|earlier| earlier != artifact)
                {
                    self.reached.approvals_replaced += 1;
                }
                self.oracle.approved = Some(artifact);
            }
            Decision::Reject => self.oracle.status = Status::Rejected,
        }
        assert_eq!(body["status"], self.oracle.status.wire(), "{step}: {body}");
    }

    async fn reopen(&mut self, step: &str) {
        for status in ["archived", "draft"] {
            let (answer, body) = call(
                self.f,
                &self.f.author.token,
                "PATCH",
                &format!("/api/v1/missions/{}", self.mission),
                Some(json!({ "status": status })),
            )
            .await;
            assert_eq!(answer, StatusCode::OK, "{step}: {body}");
            assert_eq!(body["status"], status, "{step}: {body}");
        }
        self.oracle.status = Status::Draft;
    }

    async fn request_deployment(&mut self, step: &str, target: Target) {
        let artifact = self.oracle.resolve(target);
        let answer = call(
            self.f,
            &self.f.admin.token,
            "POST",
            &format!("/api/v1/servers/{}/deployments", self.server),
            Some(json!({ "mission_id": self.mission, "artifact_id": artifact })),
        )
        .await;
        if let Some((in_flight, _)) = self.oracle.in_flight {
            assert_refused(
                step,
                &answer,
                StatusCode::CONFLICT,
                "DEPLOYMENT_IN_PROGRESS",
            );
            assert_eq!(uuid_of(&answer.1["details"]["deployment_id"]), in_flight);
            return;
        }
        if self.oracle.status != Status::Live || self.oracle.approved != Some(artifact) {
            assert_refused(step, &answer, StatusCode::CONFLICT, "ARTIFACT_NOT_APPROVED");
            if self.oracle.status == Status::Live && self.oracle.artifacts.contains(&artifact) {
                self.reached.unapproved_artifacts_of_live_missions_refused += 1;
            }
            return;
        }
        let (status, body) = answer;
        assert_eq!(status, StatusCode::ACCEPTED, "{step}: {body}");
        assert_eq!(uuid_of(&body["artifact_id"]), artifact, "{step}: {body}");
        assert_eq!(
            body["artifact_sha256"], self.artifact_rows[&artifact].1,
            "{step}: the deployment carries the approved artifact's bytes: {body}"
        );
        assert_eq!(body["state"], "requested", "{step}: {body}");
        let deployment = uuid_of(&body["id"]);
        self.oracle.in_flight = Some((deployment, artifact));
        self.oracle.deployed = Some(artifact);
        self.reached.deployments_accepted += 1;
        self.oracle
            .deployments
            .insert(deployment, (artifact, "requested"));
    }

    async fn report_loaded(&mut self, step: &str, target: Target, exact_bytes: bool) {
        let artifact = self.oracle.resolve(target);
        let sha256 = match self.artifact_rows.get(&artifact) {
            Some((_, sha256)) if exact_bytes => sha256.clone(),
            _ => sha256_hex(format!("bytes no artifact has {}", Uuid::new_v4()).as_bytes()),
        };
        let answer = call(
            self.f,
            &self.runtime_secret,
            "POST",
            "/api/v1/game-runtime/sessions",
            Some(json!({ "loaded_artifact_id": artifact, "loaded_artifact_sha256": sha256 })),
        )
        .await;
        if !self.oracle.artifacts.contains(&artifact) {
            assert_refused(
                step,
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "UNKNOWN_ARTIFACT",
            );
            return;
        }
        let (status, body) = answer;
        assert_eq!(status, StatusCode::CREATED, "{step}: {body}");
        let session = uuid_of(&body["runtime_session_id"]);
        let Some((deployment, deployed)) = self.oracle.in_flight.take() else {
            return;
        };
        let (answer, settled) = call(
            self.f,
            &self.f.admin.token,
            "GET",
            &format!("/api/v1/servers/{}/deployments/{deployment}", self.server),
            None,
        )
        .await;
        assert_eq!(answer, StatusCode::OK, "{step}: {settled}");
        let state = if artifact == deployed && exact_bytes {
            assert_eq!(
                uuid_of(&settled["confirmed_runtime_session_id"]),
                session,
                "{step}: {settled}"
            );
            self.reached.deployments_confirmed += 1;
            "confirmed"
        } else {
            self.reached.deployments_failed += 1;
            "failed"
        };
        assert_eq!(settled["state"], state, "{step}: {settled}");
        self.oracle
            .deployments
            .insert(deployment, (deployed, state));
    }

    /// The persisted rows agree with the oracle, and every artifact row is unchanged.
    async fn check_invariants(&mut self, step: &str) {
        let pool = self.f.pool();
        let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
            "SELECT id, md5(a::text), document_sha256, encode(sha256(document), 'hex')
             FROM mission_artifacts a WHERE mission_id = $1",
        )
        .bind(self.mission)
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(
            rows.len(),
            self.oracle.artifacts.len(),
            "{step}: only submissions write artifacts"
        );
        for (id, fingerprint, sha256, computed) in rows {
            assert!(self.oracle.artifacts.contains(&id), "{step}: artifact {id}");
            assert_eq!(sha256, computed, "{step}: artifact {id} records its bytes");
            let first = self
                .artifact_rows
                .entry(id)
                .or_insert((fingerprint.clone(), sha256));
            assert_eq!(first.0, fingerprint, "{step}: artifact {id} changed");
        }
        let mission: (String, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
            "SELECT status::text, approved_artifact_id, current_version_id FROM missions WHERE id = $1",
        )
        .bind(self.mission)
        .fetch_one(pool)
        .await
        .unwrap();
        let expected = &self.oracle;
        assert_eq!(
            mission,
            (
                expected.status.wire().to_owned(),
                expected.approved,
                expected.current_version
            ),
            "{step}: mission status, approved artifact and current version"
        );
        let pending: Vec<Uuid> = sqlx::query_scalar(
            "SELECT artifact_id FROM mission_reviews WHERE mission_id = $1 AND state = 'pending'",
        )
        .bind(self.mission)
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(pending, Vec::from_iter(expected.under_review), "{step}");
        let deployments: Vec<DeploymentRow> = sqlx::query_as(DEPLOYMENT_ROWS)
            .bind(self.server)
            .fetch_all(pool)
            .await
            .unwrap();
        assert_eq!(deployments.len(), expected.deployments.len(), "{step}");
        for row in deployments {
            let id = row.id;
            let (artifact, state) = expected.deployments[&id];
            assert_eq!(
                (row.mission_id, row.artifact_id, row.state.as_str()),
                (self.mission, artifact, state),
                "{step}: deployment {id}"
            );
            assert_eq!(
                row.approval_in_force,
                Some(artifact),
                "{step}: deployment {id} names the artifact of the latest approval"
            );
            assert_eq!(
                (row.command_artifact, row.command_sha256),
                (
                    Some(artifact.to_string()),
                    Some(row.document_sha256.clone())
                ),
                "{step}: deployment {id}'s fleet command carries its artifact and bytes"
            );
            let confirmed_by = (row.loaded_artifact_id, row.loaded_artifact_sha256);
            if state == "confirmed" {
                assert_eq!(
                    confirmed_by,
                    (Some(artifact), Some(row.document_sha256)),
                    "{step}: deployment {id} is confirmed by a report of its exact artifact"
                );
            } else {
                assert_eq!(confirmed_by, (None, None), "{step}: deployment {id}");
            }
        }
    }

    /// The operators' deployment listing answers every deployment with the artifact and bytes the
    /// oracle recorded.
    async fn check_deployment_listing(&self) {
        let (status, listing) = call(
            self.f,
            &self.f.admin.token,
            "GET",
            &format!("/api/v1/servers/{}/deployments?limit=100", self.server),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{listing}");
        let items = listing["items"].as_array().unwrap();
        assert_eq!(items.len(), self.oracle.deployments.len(), "{listing}");
        for item in items {
            let (artifact, state) = self.oracle.deployments[&uuid_of(&item["id"])];
            assert_eq!(uuid_of(&item["artifact_id"]), artifact, "{item}");
            assert_eq!(
                item["artifact_sha256"], self.artifact_rows[&artifact].1,
                "{item}"
            );
            assert_eq!(item["state"], state, "{item}");
        }
    }
}

#[test]
fn approval_and_deployment_share_immutable_artifact() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (fixture, modpack) = runtime.block_on(async {
        let fixture = MissionFixture::new(SUITE).await;
        fixture.register_scenario("everon", EVERON_SCENARIO).await;
        // Every artifact of this binary compiles against one catalog, so one probe names the
        // modpack each case's server requires.
        let (_, probe) = fixture
            .approved_mission("Artifact property modpack probe")
            .await;
        let modpack: Option<Uuid> =
            sqlx::query_scalar("SELECT modpack_id FROM mission_artifacts WHERE id = $1")
                .bind(probe)
                .fetch_one(fixture.pool())
                .await
                .unwrap();
        (fixture, modpack)
    });
    let reached = std::cell::RefCell::new(Reached::default());
    common::property_evidence::run_property(
        "approval_and_deployment_share_immutable_artifact",
        256,
        &lifecycle(),
        |operations| {
            runtime.block_on(async {
                let mut case = Case::open(&fixture, modpack).await;
                case.check_invariants("setup").await;
                for (index, operation) in operations.into_iter().enumerate() {
                    let step = format!("step {index} {operation:?}");
                    case.apply(&step, operation).await;
                    case.check_invariants(&step).await;
                }
                case.check_deployment_listing().await;
                reached.borrow_mut().absorb(&case.reached);
            });
            Ok(())
        },
    );
    let reached = reached.into_inner();
    println!("approval_and_deployment_share_immutable_artifact reached: {reached:?}");
    reached.assert_every_outcome_reached();
}
