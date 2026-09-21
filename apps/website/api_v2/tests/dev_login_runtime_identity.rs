//! Dev-login observed through the running handler: the SPA redirect, the default role, the
//! per-role identities, and the first-create `arma_id` COALESCE — every one of them read back
//! from the database or over HTTP.
//!
//! Dead code writes no rows, so a match arm that is present in the source but never compiled
//! fails here by construction. That is what these tests buy over the source scanners in
//! `tests/dev_login_source_contract.rs`.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::core::database;

mod common;
mod router_boot_support;

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

#[tokio::test]
async fn dev_login_unknown_role_defaults_to_admin() {
    let Some(app) = boot().await else { return };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=wizard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap();
    // The minted identity is an admin → /me reports role admin.
    let me = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["user"]["role"], "admin");
}

// ════════════════ dev-login pinned by BEHAVIOUR, not by source ════════════════
//
// `common/mod.rs`'s COALESCE pin and `dev_login_roles_use_distinct_discord_ids` in
// `tests/dev_login_source_contract.rs` both read
// `src/identity_and_access/handlers/developer_login.rs` as text. Every scrub-then-grep view can
// be walked around: `#[cfg(any())]` on the live match arms, and a nested `fn dev_login`, are
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
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND, "dev-login {role}");
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    let me = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    let discord_id = v["user"]["discord_id"].as_str().unwrap().to_string();
    let reported_role = v["user"]["role"].as_str().unwrap().to_string();
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

/// **The COALESCE first-create contract, observed on the row.**
///
/// The source pin under `common/` greps `dev_login` for
/// `UPDATE users SET arma_id = COALESCE(arma_id, $2)`. A nested `fn dev_login` decoy and a
/// `$decoy$…$decoy$` payload both got that green; both are guarded there, and both guards are
/// lexical, so the next walk-around only has to reach for a `cfg` or an `if false`.
///
/// `COALESCE(arma_id, $2)` means exactly two observable things, and this asserts both:
///
/// * **NULL is filled.** That is the state the handler's own INSERT leaves (it binds NULL, and
///   its `ON CONFLICT` branch does not touch the column), so a role's first-ever dev-login must
///   come out with that role's arma id.
/// * **A set value is kept.** A user who has linked a real Arma id must still have it after the
///   next dev-login. This is the half `SET arma_id = $2` gets wrong.
///
/// Either observation alone is satisfiable by a handler with no UPDATE at all; together they are
/// not. Perturbation RED:
/// - delete the UPDATE, or wrap it in `#[cfg(any())]` / `#[cfg(all(unix, any()))]` / `if false` →
///   step (A) fails: `arma_id` is still NULL.
/// - `SET arma_id = $2` → step (B) fails: the linked id was overwritten.
#[tokio::test]
async fn dev_login_first_create_coalesces_arma_id() {
    let Some(app) = boot().await else {
        eprintln!("skip: test database URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = database::connect(&url).await.expect("connect");

    // `leader` is this binary's spare role — no sibling test asserts on its row.
    const ROLE: &str = "leader";
    const ID: &str = "000000000000000003";
    const ROLE_ARMA: &str = "dev-arma-76561190000000003";

    // (A) NULL arma_id — the post-INSERT state — must be stamped by the first-create UPDATE.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE discord_id = $1")
        .bind(ID)
        .execute(&pool)
        .await
        .expect("clear arma_id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(ROLE_ARMA),
        "dev-login left `{ID}` with no arma id. The first-create \
         `UPDATE users SET arma_id = COALESCE(arma_id, $2)` did not execute — deleted, `cfg`'d \
         out, wrapped in a constant-false block, or parked in a dead sibling helper. Source \
         still containing the statement is not the contract; this row is."
    );

    // (B) An already-linked arma id must survive. This is what COALESCE buys and what
    //     `SET arma_id = $2` destroys.
    let linked = common::unique_arma("runtime-linked");
    sqlx::query("UPDATE users SET arma_id = $2 WHERE discord_id = $1")
        .bind(ID)
        .bind(&linked)
        .execute(&pool)
        .await
        .expect("link a real arma id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(linked.as_str()),
        "dev-login overwrote a linked arma id. `COALESCE(arma_id, $2)` keeps what is \
         already there; a bare `SET arma_id = $2` is the regression this pins."
    );

    // Restore the row to the role's own id so a sibling test reads the shipped shape.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE discord_id = $1")
        .bind(ID)
        .execute(&pool)
        .await
        .expect("clear arma_id");
    dev_login_identity(&app, ROLE).await;
    assert_eq!(
        arma_id_of(&pool, ID).await.as_deref(),
        Some(ROLE_ARMA),
        "re-stamp after unlink must reach the role's own arma id"
    );
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
    let pool = database::connect(&url).await.expect("connect");

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

/// Live: enlisted then admin must not rewrite each other's row / collide identity.
///
/// Perturbation RED: restore a single shared `DEV_USER_ID` for every role → after enlisted
/// login, admin login leaves one row whose role is admin, and enlisted's `/me` (JWT still
/// carries the shared id) reports admin — or the two `/me` discord_ids equal.
#[tokio::test]
async fn dev_login_roles_do_not_rewrite_each_other() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let _guard = DEV_ROLE_ROWS.lock().await;
    let url = common::require_test_database_url().expect("boot succeeded ⇒ URL set");
    let pool = database::connect(&url).await.expect("connect");

    let (_e_tok, e_id, e_role) = dev_login_identity(&app, "enlisted").await;
    assert_eq!(e_role, "enlisted");
    assert_eq!(e_id, "000000000000000002");

    let (_a_tok, a_id, a_role) = dev_login_identity(&app, "admin").await;
    assert_eq!(a_role, "admin");
    assert_eq!(a_id, "000000000000000001");
    assert_ne!(e_id, a_id, "roles must not share a discord_id");

    // Enlisted row must still be enlisted after admin login (the shared-row rewrite).
    let enlisted_db_role: String =
        sqlx::query_scalar("SELECT role::text FROM users WHERE discord_id = $1")
            .bind(&e_id)
            .fetch_one(&pool)
            .await
            .expect("enlisted row");
    assert_eq!(
        enlisted_db_role, "enlisted",
        "admin login must not rewrite the enlisted row's role"
    );

    let (_m_tok, m_id, m_role) = dev_login_identity(&app, "mission_maker").await;
    assert_eq!(m_role, "mission_maker");
    assert_eq!(m_id, "000000000000000004");
    assert_ne!(m_id, e_id);
    assert_ne!(m_id, a_id);
}
