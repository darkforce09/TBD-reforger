//! Shared plumbing for the telemetry ingest suites.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per **top-level** `tests/*.rs` file. Files under a
//! `tests/<dir>/` subdirectory are not auto-discovered, so this module is compiled *into*
//! whichever suite writes `mod telemetry_support;` and adds no target of its own — the same
//! rule that makes `tests/common/mod.rs` the conventional home for shared test code.
//!
//! # What lives here
//!
//! * [`SVC`] — the service token the ingest routes authenticate with. Ingest is machine
//!   traffic, so it carries no bearer; the read-back routes the suites assert against do.
//! * [`boot`] — a router over this binary's own database (see [`common::require_test_database_url`]),
//!   or `None` when `TEST_DATABASE_URL` is unset, which is the suite-skip path.
//! * [`admin_token`] — a bearer for the read-back routes, via the dev-login handler.
//! * [`call`] — one request, both credential kinds optional. A call with neither is how a
//!   suite asserts the 401, so they are separate arguments rather than one enum.
//!
//! Every suite that includes this module gets its own database and its own in-process rate
//! limiter, because both are keyed per test binary: the limiter buckets on the peer IP, which
//! is `0.0.0.0` for every request a binary makes, so the ingest calls of all its tests share
//! one 1/s + burst-10 bucket.

// Each telemetry binary compiles its own copy of this module and names a different subset of
// it, so an item one suite never calls is not dead code — but rustc judges each binary on its
// own, and the gate runs `clippy -p website-api --all-targets -- -D warnings`.
#![allow(dead_code)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

use crate::common;

/// The service token `Config::for_tests` accepts on the ingest routes.
pub const SVC: &str = "test-service-token";

/// Router + pool over this binary's private database, or `None` when the suite must skip.
pub async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let app = http_router::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "tele-secret"),
    ));
    Some((app, pool))
}

/// An admin bearer for the status / leaderboard / stats read-backs.
pub async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "telemetry", "admin").await
}

/// One request against the router, with an optional bearer and an optional service token.
///
/// Both are optional because the ingest contract is asserted from both sides: a service-token
/// call is the game server, a bearer call is an operator reading the result back, and a call
/// with neither must be a 401.
pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    svc: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = svc {
        b = b.header("x-service-token", s);
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A started runtime session of one registered server and the credential that started it.
pub struct RuntimeSession {
    pub secret: String,
    pub id: uuid::Uuid,
    pub generation: i64,
}

/// Issue a `mod_runtime` credential for `server` and start its next runtime session.
pub async fn runtime_session(app: &Router, pool: &PgPool, server: uuid::Uuid) -> RuntimeSession {
    let credential = uuid::Uuid::new_v4();
    let secret = format!(
        "tbdm_{}_{}",
        credential.simple(),
        website_api::core::authentication_primitives::random_token(32)
    );
    sqlx::query(
        "INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'mod_runtime', $3, 'Telemetry runtime', $4)",
    )
    .bind(credential)
    .bind(server)
    .bind(website_api::core::authentication_primitives::hash_token(&secret))
    .bind(common::DEV_LOGIN_USER)
    .execute(pool)
    .await
    .expect("store runtime credential");
    let (status, started) = call(
        app,
        "POST",
        "/api/v1/game-runtime/sessions",
        Some(&secret),
        None,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "start runtime session: {started}"
    );
    RuntimeSession {
        id: started["runtime_session_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        generation: started["generation"].as_i64().unwrap(),
        secret,
    }
}

/// One heartbeat of `session` with `sequence`; `reading` holds the status fields.
pub async fn heartbeat(
    app: &Router,
    session: &RuntimeSession,
    sequence: i64,
    reading: Value,
) -> (StatusCode, Value) {
    let mut body = serde_json::json!({ "generation": session.generation, "sequence": sequence });
    for (key, value) in reading.as_object().expect("a reading is a JSON object") {
        body[key] = value.clone();
    }
    call(
        app,
        "POST",
        &format!("/api/v1/game-runtime/sessions/{}/heartbeats", session.id),
        Some(&session.secret),
        None,
        Some(&body.to_string()),
    )
    .await
}

/// Remove a server and everything its runtime left behind.
pub async fn remove_server(pool: &PgPool, server: uuid::Uuid) {
    for statement in [
        "DELETE FROM server_status_histories WHERE server_id = $1",
        "DELETE FROM server_statuses WHERE server_id = $1",
        "DELETE FROM server_runtime_sessions WHERE server_id = $1",
        "DELETE FROM server_machine_credentials WHERE server_id = $1",
        "DELETE FROM servers WHERE id = $1",
    ] {
        sqlx::query(statement)
            .bind(server)
            .execute(pool)
            .await
            .unwrap_or_else(|error| panic!("`{statement}`: {error}"));
    }
}
