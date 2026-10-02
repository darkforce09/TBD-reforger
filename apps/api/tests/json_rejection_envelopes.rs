//! An unreadable JSON body answers in the `{error, details?}` envelope with the status of its
//! cause on every write that decodes a typed body.
//!
//! **Role:** one case per handler that decodes its body before it reads any stored row: the
//! route's least caller sends `{}` without a content type (415) and a body one byte over the
//! JSON body limit (413 with `details.code = request_too_large`). Handlers that load a stored row
//! first (the mission patch, armory and versions, the squad and slot writes, the event mission
//! attachment, the fleet executor and the event access writes) are proven by the derived
//! malformed and boundary probes of the `route_acceptance_*` binaries, whose worlds supply real
//! rows. The administrator ballistics catalog upload reads a multipart form under its own body
//! limit, so its case sends a JSON body (415) and a form one byte over that limit (413).
//! **Position:** boots `core::http_router::router` over this binary's own test database with the
//! development test configuration; user sessions come from `common::access_token`, and a
//! `mod_runtime` machine credential is issued through the administrator credential route for a
//! server row this binary inserts. The handlers reach the envelope through
//! `ApiError::from_json_rejection`.
//! **Signals & state:** none beyond the per-binary database; every actor and server is a
//! suite-owned row named after its case, and every request carries a fresh synthetic peer so the
//! rate limiter never answers in the handler's place.
//! **Invariants:** a flat 400 for either probe fails the case, as does a refusal that is not a
//! JSON object with a string `error`, or a 413 without `details.code = request_too_large`.

mod common;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use api::core::application_state::AppState;
use api::core::configuration::Config;
use api::core::database;
use api::core::http_router;
use api::core::middleware::MAX_JSON_BODY;
use api::operations::handlers::ballistics_catalogs::upload::MAX_CATALOG_UPLOAD_BODY_BYTES;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

const SUITE: &str = "json_rejection_envelopes";

/// Any id: every case's handler refuses the body before it reads a stored row.
const PATH_ID: &str = "7c1d9e2a-4b3f-4e8a-9d6c-5a2b1f0e3c47";

/// The caller a case sends its requests as.
enum Caller {
    /// No bearer: the route authenticates through the body's own credential.
    Anonymous,
    /// A signed-in actor of this role, the least one that reaches the handler.
    Role(&'static str),
    /// A `mod_runtime` machine credential of a server this binary registers.
    ModRuntime,
}

/// One body write: the area its test is named after, its method and path, and its caller.
struct BodyCase {
    area: &'static str,
    method: &'static str,
    path: &'static str,
    caller: Caller,
}

const CASES: [BodyCase; 33] = [
    BodyCase {
        area: "admin_user_role",
        method: "PATCH",
        path: "/api/v1/admin/users/json-rejection-target",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "admin_user_ban",
        method: "POST",
        path: "/api/v1/admin/users/json-rejection-target/ban",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "admin_user_warning",
        method: "POST",
        path: "/api/v1/admin/users/json-rejection-target/warnings",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "admin_user_membership_grace",
        method: "POST",
        path: "/api/v1/admin/users/json-rejection-target/membership-grace",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "modpack_create",
        method: "POST",
        path: "/api/v1/modpacks",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "modpack_replace",
        method: "PUT",
        path: "/api/v1/modpacks/{id}",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "approval_approve",
        method: "POST",
        path: "/api/v1/approvals/{id}/approve",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "approval_reject",
        method: "POST",
        path: "/api/v1/approvals/{id}/reject",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "server_deployment_request",
        method: "POST",
        path: "/api/v1/servers/{id}/deployments",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "server_create",
        method: "POST",
        path: "/api/v1/servers",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "server_patch",
        method: "PATCH",
        path: "/api/v1/servers/{id}",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "server_command",
        method: "POST",
        path: "/api/v1/servers/{id}/commands",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "server_credential",
        method: "POST",
        path: "/api/v1/servers/{id}/credentials",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "fleet_scenario",
        method: "PUT",
        path: "/api/v1/fleet/scenarios/everon",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "event_create",
        method: "POST",
        path: "/api/v1/events",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "event_patch",
        method: "PATCH",
        path: "/api/v1/events/{id}",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "leave_review",
        method: "PATCH",
        path: "/api/v1/admin/leave-requests/{id}",
        caller: Caller::Role("admin"),
    },
    BodyCase {
        area: "faction_create",
        method: "POST",
        path: "/api/v1/factions",
        caller: Caller::Role("mission_maker"),
    },
    BodyCase {
        area: "faction_replace",
        method: "PUT",
        path: "/api/v1/factions/{id}",
        caller: Caller::Role("mission_maker"),
    },
    BodyCase {
        area: "mission_create",
        method: "POST",
        path: "/api/v1/missions",
        caller: Caller::Role("mission_maker"),
    },
    BodyCase {
        area: "mission_review_comment",
        method: "POST",
        path: "/api/v1/missions/{id}/review-comments",
        caller: Caller::Role("mission_maker"),
    },
    BodyCase {
        area: "fire_mission_save",
        method: "POST",
        path: "/api/v1/fire-missions",
        caller: Caller::Role("enlisted"),
    },
    BodyCase {
        area: "leave_submit",
        method: "POST",
        path: "/api/v1/me/leave-requests",
        caller: Caller::Role("enlisted"),
    },
    BodyCase {
        area: "session_refresh",
        method: "POST",
        path: "/api/v1/auth/refresh",
        caller: Caller::Anonymous,
    },
    BodyCase {
        area: "session_logout",
        method: "POST",
        path: "/api/v1/auth/logout",
        caller: Caller::Anonymous,
    },
    BodyCase {
        area: "link_confirm",
        method: "POST",
        path: "/api/v1/ingest/link-confirm",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "match_registration",
        method: "POST",
        path: "/api/v1/ingest/matches",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "match_event_batch",
        method: "POST",
        path: "/api/v1/ingest/match-events",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "match_results",
        method: "POST",
        path: "/api/v1/ingest/match-results",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "runtime_session_start",
        method: "POST",
        path: "/api/v1/game-runtime/sessions",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "runtime_heartbeat",
        method: "POST",
        path: "/api/v1/game-runtime/sessions/{id}/heartbeats",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "runtime_session_deployment",
        method: "POST",
        path: "/api/v1/game-runtime/sessions/{id}/deployments",
        caller: Caller::ModRuntime,
    },
    BodyCase {
        area: "runtime_relayed_deployment",
        method: "POST",
        path: "/api/v1/game-runtime/deployments",
        caller: Caller::ModRuntime,
    },
];

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request in this binary has used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 43000))
}

/// Send one request and answer its status and body bytes.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(bearer) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    }
    if let Some(content_type) = content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    let mut request = builder.body(Body::from(body)).expect("request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, bytes.to_vec())
}

/// Issue a `mod_runtime` credential for a fresh server row and answer its secret.
async fn mod_runtime_secret(app: &Router, state: &AppState, area: &str) -> String {
    let server: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) \
         VALUES ($1, '127.0.0.1'::inet, 2001, true) RETURNING id",
    )
    .bind(format!("json-rejection-{area}"))
    .fetch_one(&state.pool)
    .await
    .expect("insert server");
    let admin = common::access_token(
        state,
        SUITE,
        &format!("json-rejection-issuer-{area}"),
        "admin",
        true,
    )
    .await;
    let request = json!({"executor_kind": "mod_runtime", "label": format!("{area} runtime")});
    let (status, bytes) = send(
        app,
        "POST",
        &format!("/api/v1/servers/{server}/credentials"),
        Some(&admin),
        Some("application/json"),
        request.to_string().into_bytes(),
    )
    .await;
    let body: Value = serde_json::from_slice(&bytes).expect("credential JSON");
    assert_eq!(status, StatusCode::CREATED, "credential issue: {body}");
    body["secret"].as_str().expect("secret").to_owned()
}

/// Require a refusal of `status` in the JSON envelope; answer the envelope.
fn envelope(label: &str, status: StatusCode, expected: StatusCode, bytes: &[u8]) -> Value {
    let text = String::from_utf8_lossy(bytes);
    assert_eq!(
        status, expected,
        "{label}: expected {expected}, body {text}"
    );
    let body: Value = serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!("{label}: the refusal is not the JSON error envelope ({e}); body: {text}")
    });
    assert!(
        body.get("error")
            .and_then(Value::as_str)
            .is_some_and(|e| !e.is_empty()),
        "{label}: the refusal has no string `error`; body: {body}"
    );
    body
}

/// Send the case's missing-content-type and over-limit bodies and require 415 and 413.
async fn assert_json_rejection_envelopes(area: &str) {
    let case = CASES
        .iter()
        .find(|case| case.area == area)
        .unwrap_or_else(|| panic!("no body case for area `{area}`"));
    let url = common::require_test_database_url()
        .expect("the per-binary test database is provisioned before any case runs");
    let pool = database::connect(&url).await.expect("connect");
    let state = AppState::new(pool, Config::for_tests(url, "json-rejection-secret"));
    let app = http_router::router(state.clone());
    let bearer = match case.caller {
        Caller::Anonymous => None,
        Caller::Role(role) => {
            let actor = format!("json-rejection-{}", case.area);
            Some(common::access_token(&state, SUITE, &actor, role, true).await)
        }
        Caller::ModRuntime => Some(mod_runtime_secret(&app, &state, case.area).await),
    };
    let path = case.path.replace("{id}", PATH_ID);
    let label = format!("{} {path}", case.method);

    let (status, bytes) = send(
        &app,
        case.method,
        &path,
        bearer.as_deref(),
        None,
        b"{}".to_vec(),
    )
    .await;
    envelope(
        &format!("{label} without a content type"),
        status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        &bytes,
    );

    let mut over_limit = br#"{"padding":""#.to_vec();
    over_limit.resize(MAX_JSON_BODY + 1, b'a');
    over_limit.extend_from_slice(br#""}"#);
    let (status, bytes) = send(
        &app,
        case.method,
        &path,
        bearer.as_deref(),
        Some("application/json"),
        over_limit,
    )
    .await;
    let body = envelope(
        &format!("{label} over the body limit"),
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        &bytes,
    );
    assert_eq!(
        body["details"]["code"], "request_too_large",
        "{label} over the body limit: {body}"
    );
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_admin_user_role() {
    assert_json_rejection_envelopes("admin_user_role").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_admin_user_ban() {
    assert_json_rejection_envelopes("admin_user_ban").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_admin_user_warning() {
    assert_json_rejection_envelopes("admin_user_warning").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_admin_user_membership_grace() {
    assert_json_rejection_envelopes("admin_user_membership_grace").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_modpack_create() {
    assert_json_rejection_envelopes("modpack_create").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_modpack_replace() {
    assert_json_rejection_envelopes("modpack_replace").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_approval_approve() {
    assert_json_rejection_envelopes("approval_approve").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_approval_reject() {
    assert_json_rejection_envelopes("approval_reject").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_server_deployment_request() {
    assert_json_rejection_envelopes("server_deployment_request").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_server_create() {
    assert_json_rejection_envelopes("server_create").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_server_patch() {
    assert_json_rejection_envelopes("server_patch").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_server_command() {
    assert_json_rejection_envelopes("server_command").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_server_credential() {
    assert_json_rejection_envelopes("server_credential").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_fleet_scenario() {
    assert_json_rejection_envelopes("fleet_scenario").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_event_create() {
    assert_json_rejection_envelopes("event_create").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_event_patch() {
    assert_json_rejection_envelopes("event_patch").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_leave_review() {
    assert_json_rejection_envelopes("leave_review").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_faction_create() {
    assert_json_rejection_envelopes("faction_create").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_faction_replace() {
    assert_json_rejection_envelopes("faction_replace").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_mission_create() {
    assert_json_rejection_envelopes("mission_create").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_mission_review_comment() {
    assert_json_rejection_envelopes("mission_review_comment").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_fire_mission_save() {
    assert_json_rejection_envelopes("fire_mission_save").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_leave_submit() {
    assert_json_rejection_envelopes("leave_submit").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_session_refresh() {
    assert_json_rejection_envelopes("session_refresh").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_session_logout() {
    assert_json_rejection_envelopes("session_logout").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_link_confirm() {
    assert_json_rejection_envelopes("link_confirm").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_match_registration() {
    assert_json_rejection_envelopes("match_registration").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_match_event_batch() {
    assert_json_rejection_envelopes("match_event_batch").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_match_results() {
    assert_json_rejection_envelopes("match_results").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_runtime_session_start() {
    assert_json_rejection_envelopes("runtime_session_start").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_runtime_heartbeat() {
    assert_json_rejection_envelopes("runtime_heartbeat").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_runtime_session_deployment() {
    assert_json_rejection_envelopes("runtime_session_deployment").await;
}

#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_runtime_relayed_deployment() {
    assert_json_rejection_envelopes("runtime_relayed_deployment").await;
}

/// The catalog upload: a JSON body answers 415 and a form one byte over the route's own limit
/// 413, both in the envelope.
#[tokio::test]
async fn contract_parity_json_rejections_answer_the_error_envelope_ballistics_catalog_upload() {
    let url = common::require_test_database_url()
        .expect("the per-binary test database is provisioned before any case runs");
    let pool = database::connect(&url).await.expect("connect");
    let state = AppState::new(pool, Config::for_tests(url, "json-rejection-secret"));
    let app = http_router::router(state.clone());
    let actor = "json-rejection-ballistics_catalog_upload";
    let bearer = common::access_token(&state, SUITE, actor, "admin", true).await;
    let path = "/api/v1/ballistics-catalogs";
    let label = format!("POST {path}");

    let (status, bytes) = send(
        &app,
        "POST",
        path,
        Some(&bearer),
        Some("application/json"),
        b"{}".to_vec(),
    )
    .await;
    envelope(
        &format!("{label} with a JSON body"),
        status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        &bytes,
    );

    // A part the route skips, so no part cap answers before the route's body limit does.
    let boundary = "json-rejection-catalog-upload";
    let head = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"padding\"; \
         filename=\"padding.json\"\r\nContent-Type: application/json\r\n\r\n"
    );
    let tail = format!("\r\n--{boundary}--\r\n");
    let mut over_limit = head.into_bytes();
    over_limit.resize(MAX_CATALOG_UPLOAD_BODY_BYTES + 1 - tail.len(), b' ');
    over_limit.extend_from_slice(tail.as_bytes());
    assert_eq!(over_limit.len(), MAX_CATALOG_UPLOAD_BODY_BYTES + 1);
    let (status, bytes) = send(
        &app,
        "POST",
        path,
        Some(&bearer),
        Some(&format!("multipart/form-data; boundary={boundary}")),
        over_limit,
    )
    .await;
    let body = envelope(
        &format!("{label} over the body limit"),
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        &bytes,
    );
    assert_eq!(
        body["details"]["code"], "request_too_large",
        "{label} over the body limit: {body}"
    );
}
