//! Dev-login observed through the running handler: the SPA redirect, the default role, the
//! per-role identities, and the first-create `arma_id` COALESCE — every one of them read back
//! from the database or over HTTP.
//!
//! Dead code writes no rows, so a match arm that is present in the source but never compiled
//! fails here by construction.
//!
//! Skips without `TEST_DATABASE_URL`.

use crate::{common, router_boot_support};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use router_boot_support::*;

#[tokio::test]
async fn dev_login_redirects_to_spa() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    assert!(loc.starts_with(ORIGIN), "redirects to SPA: {loc}");
    assert!(loc.contains("access_token="), "carries the token fragment");
}

// ════════════════ dev-login pinned by BEHAVIOUR, not by source ════════════════
//
// A text scan of `crates/api/api_identity_and_access/src/handlers/developer_login.rs` can be
// walked around: `#[cfg(any())]` on the live match arms, and a nested `fn dev_login`, are
// questions about **reachability**, which no grep can answer.
//
// The three tests below answer it by running the handler against a real database. Dead code
// writes no rows, so every wrapper — the ones already invented and the ones nobody has — fails
// here by construction. The source pins stay as fast first failures; these are the contract.
//
// They share the per-role `users` rows, so they take one lock rather than reasoning about windows.
static DEV_ROLE_ROWS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `GET /auth/dev-login?role=…` → `(access token, /me discord_id, /me role)`.
///
/// Asserts the 302 itself: a 500 here is the `idx_users_arma_id` unique violation that a
/// collapsed `arma_id_for_role` produces on the second role's cold first create.
async fn dev_login_identity(app: &Router, role: &str) -> (String, String, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .expect("the request builds"),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND, "dev-login {role}");
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .expect("the Location header is ASCII")
        .to_string();
    let tok = loc
        .split_once('#')
        .expect("the Location carries a fragment")
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .expect("the fragment carries an access token")
        .to_string();
    let me = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .expect("the request builds"),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    let v: Value = serde_json::from_slice(&body).expect("the body decodes as JSON");
    let discord_id = v["user"]["discord_id"]
        .as_str()
        .expect("the `discord_id` field is a string")
        .to_string();
    let reported_role = v["user"]["role"]
        .as_str()
        .expect("the `role` field is a string")
        .to_string();
    (tok, discord_id, reported_role)
}

/// `users.arma_id` for a discord id — `None` for "no row" and for "row with NULL".
async fn arma_id_of(pool: &PgPool, discord_id: &str) -> Option<String> {
    let row: Option<Option<String>> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .expect("read users.arma_id");
    row.flatten()
}

/// **Every role's identity, observed over HTTP.**
///
/// `dev_login_roles_use_distinct_discord_ids` greps `discord_id_for_role` /
/// `arma_id_for_role` for their live arms. That stays green under `#[cfg(any())]` on the arms
/// with a live `_ => DEV_USER_ID`: the arms are still *in the file*, so every scrub-then-grep
/// view still sees them, while the compiled `match` has one arm and every role folds back onto
/// one shared row.
///
/// This asks the running handler instead. Four dev-logins, four `/me` identities:
///
/// * cfg'd-out **discord** arms → every role reports `…001` and the first `assert_eq!` fires.
/// * cfg'd-out **arma** arms → every role wants `dev-arma-…001`, so the second role's cold
///   first create trips the `idx_users_arma_id` unique index and dev-login answers 500 — the
///   302 assertion in [`dev_login_identity`] fires. (That index is the reason per-role arma ids
///   exist at all — per-role ids and the first-create COALESCE are one contract.)
///
/// Perturbation RED: `#[cfg(any())]` on the three role arms of either helper,
/// and `#[cfg(all(unix, any()))]` on them — the second walks around a literal-`#[cfg(any())]`
/// scrubber, and neither survives here.
#[tokio::test]
async fn dev_login_gives_every_role_its_own_identity_at_runtime() {
    let Some(app) = boot().await else {
        eprintln!("skip: test database URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = api_database::connect(&url).await.expect("connect");

    const EXPECT: [(&str, &str, &str); 4] = [
        ("admin", "000000000000000001", "dev-arma-76561190000000001"),
        (
            "enlisted",
            "000000000000000002",
            "dev-arma-76561190000000002",
        ),
        ("leader", "000000000000000003", "dev-arma-76561190000000003"),
        (
            "mission_maker",
            "000000000000000004",
            "dev-arma-76561190000000004",
        ),
    ];

    let mut minted: Vec<String> = Vec::new();
    for (role, want_id, want_arma) in EXPECT {
        let (_tok, got_id, got_role) = dev_login_identity(&app, role).await;
        assert_eq!(
            got_id, want_id,
            "role `{role}` minted discord_id `{got_id}`, not its own `{want_id}`. Every \
             role folding onto one row is the defect: `ON CONFLICT` rewrites that \
             row's role out from under the other roles and `issue_session` stacks refresh \
             families on it. A `cfg`-disabled match arm reads exactly like a live one in source."
        );
        assert_eq!(got_role, role, "`/me` role for `{role}`");
        assert_eq!(
            arma_id_of(&pool, want_id).await.as_deref(),
            Some(want_arma),
            "role `{role}` must own arma id `{want_arma}` — per-role ids are what keep \
             concurrent cold first creates off each other on `idx_users_arma_id`"
        );
        minted.push(got_id);
    }

    let mut distinct = minted.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        4,
        "four roles must mint four distinct identities; got {minted:?}"
    );
}
