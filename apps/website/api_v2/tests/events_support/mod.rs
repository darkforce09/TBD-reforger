//! Shared plumbing for the event / ORBAT / registration suites.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per **top-level** `tests/*.rs` file. Files under a
//! `tests/<dir>/` subdirectory are not auto-discovered, so this module is compiled *into*
//! whichever suite writes `mod events_support;` and adds no target of its own — the same rule
//! that makes `tests/common/mod.rs` the conventional home for shared test code.
//!
//! # Why these suites seed a second and third actor
//!
//! dev-login mints a single fixed identity, so every multi-actor path — a seat already taken,
//! a squad reserved by someone else, a waitlister who must not be promoted — needs occupants
//! the handler did not create. They are seeded by direct SQL for [`OTHER`] / [`THIRD`] and
//! then driven through the real handler, which makes the race-loser branches deterministic.
//!
//! # Fixture ownership
//!
//! [`OTHER`] and [`THIRD`] sit in an id range private to these suites: no other suite, `src/`
//! or `seeds/` writes them. That matters because a fixture row another binary rewrites
//! mid-run is a failure nobody can reproduce from the failing assertion.
//!
//! [`arma`] mints a *unique* `arma_id` per call rather than deriving a fixed
//! `events-arma-{discord_id}`. `users.arma_id` carries a unique index, so one fixed string is
//! one global slot: two seeds racing for it (or a leftover foreign holder) make `seed_user`
//! panic on a duplicate key. [`DB_LOCK`] serialises the DB-touching tests of one binary on
//! top of that, since they share the two seeded actors and the rows hung off them; each test
//! binary has its own `DB_LOCK` and its own database, so the lock is only ever about the
//! tests compiled beside it.

// Each events binary compiles its own copy of this module and names a different subset of it,
// so an item one suite never calls is not dead code — but rustc judges each binary on its own,
// and the gate runs `clippy -p website-api --all-targets -- -D warnings`.
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

/// Serialise the DB-touching tests of one binary — they share [`OTHER`] / [`THIRD`] and the
/// event rows hung off them. Same pattern as `identity_link.rs` / `null_tolerance_reads.rs`.
pub static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// The second actor: the one already holding a seat when the caller claims.
///
/// Namespaced to a range these suites own outright — verified unused across the whole
/// repository (the sibling suites, `src/` and `seeds/`) before it was picked.
pub const OTHER: &str = "000000000000334002";
/// A third seeded identity — the one that must stay on the waitlist while someone else moves
/// between seats. Same private range as [`OTHER`].
pub const THIRD: &str = "000000000000334003";
/// The identity `dev-login` mints for every role
/// (`identity_and_access::handlers::developer_login::DEV_USER_ID`).
///
/// Shared with every other dev-login caller — that is inherent to the handler, not something
/// these suites can namespace away. Nothing here asserts on that row's columns; it is only
/// ever the *subject* of a request whose effects are checked in these suites' own `events` /
/// `orbat_slots` rows.
pub const DEV_USER: &str = common::DEV_LOGIN_USER;

/// A durable, unique `arma_id` for a seeded actor.
///
/// Must be unique across the whole database (`idx_users_arma_id`). A fixed
/// `events-arma-{discord_id}` is one slot and races under parallel integration runs — mint a
/// unique string instead (AtomicU64 + UUID via [`common::unique_arma`]), keeping the
/// `events-arma-{discord_id}-` prefix so a row in the database still names its owner.
pub fn arma(discord_id: &str) -> String {
    common::unique_arma(&format!("events-arma-{discord_id}"))
}

/// Router + pool over this binary's private database, or `None` when the suite must skip.
pub async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let app = http_router::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "events-secret"),
    ));
    Some((app, pool))
}

/// A bearer for `role`, minted through the real dev-login route.
pub async fn token(app: &Router, role: &str) -> String {
    common::dev_login_token(app, "events", role).await
}

/// One bearer-authenticated request against the router.
pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    tok: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
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
