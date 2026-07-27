//! Shared support for the `website-api` integration suites (T-334).
//!
//! # This file is NOT a 23rd test binary
//!
//! Cargo builds one test target per **top-level** `tests/*.rs` file. Files under a
//! `tests/<dir>/` subdirectory are not auto-discovered, so this module is compiled *into*
//! whichever suite writes `mod common;` and contributes no target of its own. Measured for
//! this crate with `cargo metadata --no-deps`: **19** `["test"]` targets before this file
//! existed and **19** after. `tests/common/mod.rs` is the conventional home precisely
//! because of that rule; `tests/common.rs` would have become a target.
//!
//! # Why it exists
//!
//! `cargo test -p website-api` builds 22 test binaries against **one** database
//! (`tbd_gate_it` under the wave gate). They run concurrently, share a schema with no
//! foreign keys, and — until this file — every one of them hand-rolled its own dev-login
//! extractor: 9 copy-pasted `async fn token`/`dev_login`/`admin_token` plus 4 inline
//! copies across 18 files, no `mod` declarations, no `[dev-dependencies]`, no `[[test]]`
//! section in `Cargo.toml`.
//!
//! The cost of that duplication is not the duplication. It is that **every copy read the
//! `Location` header through `HeaderMap`'s `Index` impl**, which panics with
//! `no entry found for key "location"` — no status, no body, and no hint at which suite or
//! actor was asking. A dev-login that 404s (route not registered) or 500s (database
//! unreachable, unique-index violation on the shared `users` row) therefore surfaces as a
//! bare key-not-found panic that names neither the cause nor the caller. Under an
//! unattended gate that reads exactly like a code failure, and a fix agent spends its whole
//! retry budget on working code.
//!
//! [`dev_login_token`] replaces that with a sentence.

// Each test binary compiles its own copy of this module and uses a different subset of it,
// so an item unused by *one* suite is not dead code — but rustc cannot know that, and the
// wave gate runs `clippy -p website-api --all-targets -- -D warnings`. Without this, adding
// a helper here for suite A turns suite B red.
#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::state::AppState;

/// Process-local counter for [`unique_arma`]. Starts at 1 so a mint never looks like a bare prefix.
static ARMA_SEQ: AtomicU64 = AtomicU64::new(1);

/// The single identity `GET /auth/dev-login` mints for **every** role
/// (`src/handlers/dev.rs:14`). It is shared by every suite that calls dev-login, and each
/// call rewrites that row's `username`, `discord_handle`, `role` and `last_login_at`
/// (`src/handlers/dev.rs:47-49`) — so a suite must never assume anything about this row
/// beyond what its own most recent dev-login call just wrote.
pub const DEV_LOGIN_USER: &str = "000000000000000001";

/// Mint an access token through `GET /api/v1/auth/dev-login?role={role}`.
///
/// `suite` is the calling file (`"events"`), `role` the tier being requested. Both appear
/// in the failure message, which is the entire point of this helper: on **any** failure it
/// reports the HTTP status, the response body, the URI and who was asking, instead of the
/// `no entry found for key "location"` index panic every hand-rolled copy produced.
///
/// # Correction: `dev_login` has no ban check (T-334, correcting T-365)
///
/// T-365 recorded that dev-login returns no `Location` header because `auth.rs:147` 403s a
/// banned account. **That mechanism is false**, and it has now cost two derivations — do not
/// derive it a third time. Verified against this tree:
///
/// * `src/handlers/dev.rs:25-64` — `dev_login` never reads `is_banned`. Its upsert *writes*
///   `is_banned = false` on insert, and the `ON CONFLICT` branch (`dev.rs:47-49`) does not
///   touch the column at all. It then calls `issue_session` unconditionally.
/// * `src/handlers/auth.rs:35-47` — `issue_session` is `issue_access` + `issue_refresh`.
///   Neither loads the user row, so `is_banned` is never consulted on this path. A banned
///   shared row still gets a **302**.
/// * `src/handlers/auth.rs:144-150` — the ban check lives in `refresh`, i.e.
///   `POST /auth/refresh`, and 403s `"account is banned"` there. That is the line T-365
///   cited; it is simply not on the dev-login path.
///
/// The real mechanism behind a missing `Location` is a **shared-fixture collision**: suites
/// mutate the shared `users` row's role, faction links and `arma_id` out from under each
/// other, and dev-login's response is then not what the next suite expects. Give your suite
/// its own actor ids (see [`seed_user`]) rather than reaching for a ban explanation.
pub async fn dev_login_token(app: &Router, suite: &str, role: &str) -> String {
    let uri = format!("/api/v1/auth/dev-login?role={role}");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .body(Body::empty())
                .expect("build dev-login request"),
        )
        .await
        .expect("dev-login must not fail below HTTP");

    // Read status + Location BEFORE consuming the body: all three go into the message.
    let status = resp.status();
    let location = resp
        .headers()
        .get(header::LOCATION)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
    let body = match to_bytes(resp.into_body(), usize::MAX).await {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => format!("<body unreadable: {e}>"),
    };
    let ctx = DevLoginFailure {
        suite,
        role,
        uri: &uri,
        status,
        body: &body,
        location: location.as_deref(),
    };

    if status != StatusCode::FOUND {
        let msg = ctx.report(
            "expected 302 Found. A non-302 means the dev-login handler did not run: 404 = \
             route not registered (Config::for_tests must set APP_ENV=development), 500 = \
             the handler's users upsert failed (unique index on users.arma_id is the usual \
             cause on a shared integration database).",
        );
        panic!("{msg}");
    }
    let Some(location) = ctx.location else {
        let msg = ctx.report(
            "302 with no Location header. The redirect was built by something other than \
             session_redirect (src/handlers/auth.rs:96-103).",
        );
        panic!("{msg}");
    };
    let Some((_, fragment)) = location.split_once('#') else {
        let msg = ctx.report(
            "Location carries no `#` fragment. auth_callback_url puts the tokens in the \
             fragment (src/handlers/auth.rs:82-94); a fragment-less Location is an error \
             redirect, and its `error=` query names the reason.",
        );
        panic!("{msg}");
    };
    let Some(token) = fragment
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
    else {
        let msg = ctx.report("the Location fragment carries no `access_token=` pair.");
        panic!("{msg}");
    };
    token.to_string()
}

/// Everything known about a failed dev-login, so the panic can name it.
struct DevLoginFailure<'a> {
    suite: &'a str,
    role: &'a str,
    uri: &'a str,
    status: StatusCode,
    body: &'a str,
    location: Option<&'a str>,
}

impl DevLoginFailure<'_> {
    fn report(&self, why: &str) -> String {
        let Self {
            suite,
            role,
            uri,
            status,
            body,
            location,
        } = *self;
        let body = if body.is_empty() {
            "<empty>".to_string()
        } else if body.len() > 2000 {
            format!("{}… ({} bytes total)", &body[..2000], body.len())
        } else {
            body.to_string()
        };
        let location = location.unwrap_or("<absent>");
        format!(
            "\n\
             ───────────────────────────────────────────────────────────────────────\n\
             dev-login did not mint a session.\n\
             \n  \
             suite:    tests/{suite}.rs\n  \
             actor:    dev-login shared user {DEV_LOGIN_USER}, role={role}\n  \
             request:  GET {uri}\n  \
             status:   {status}\n  \
             location: {location}\n  \
             body:     {body}\n\
             \n  \
             {why}\n\
             ───────────────────────────────────────────────────────────────────────"
        )
    }
}

/// Mint an `arma_id` that cannot collide with a parallel `seed_user` in this process.
///
/// `idx_users_arma_id` is a global non-partial unique index. A fixed string like
/// `events-arma-{discord_id}` is a single slot: two concurrent seeds (same or different
/// discord rows racing through release/upsert) can still trip it. Prefer this over sleep.
///
/// Format: `{prefix}-{seq}-{uuid}` — seq is monotonic in-process; uuid covers cross-binary
/// overlap on a shared gate DB (each test binary gets its own `ARMA_SEQ`).
pub fn unique_arma(prefix: &str) -> String {
    let n = ARMA_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}-{}", Uuid::new_v4())
}

/// Seed one user row this suite owns outright.
///
/// `arma_id` is a required argument and must be **unique across the whole database**:
/// `idx_users_arma_id` (`migrations/0001_initial_schema.sql:887`) is a plain
/// `CREATE UNIQUE INDEX`, not a partial one, so `''` is a *value* and only one row in the
/// entire integration database may hold it. Passing `''` here is the collision waiting to
/// happen — two suites doing it for different `discord_id`s is a unique violation on the
/// second, and `ON CONFLICT (discord_id)` does not catch it because the conflict is on the
/// other index. NULL would coexist, but a distinct string is cheaper to trace in a failure.
///
/// # T-479 — idempotent on `arma_id`
///
/// `ON CONFLICT (discord_id)` only absorbs a discord_id clash. If **another** row already
/// holds `arma_id`, the `DO UPDATE SET arma_id = EXCLUDED.arma_id` branch raises
/// `unique_violation` on `idx_users_arma_id` (observed: `event_orbat_registration_and_race`
/// / `events-arma-000000000000334002` under parallel IT). Before the upsert we release any
/// foreign holder of this `arma_id` inside the same transaction, so a leftover or racing
/// placeholder cannot poison the seed. Suites that share fixed placeholders across parallel
/// tests should still serialise (T-516 Mutex) **or** mint via [`unique_arma`].
///
/// `ON CONFLICT DO UPDATE`, not `DO NOTHING`: a suite that owns its ids wants the fixture it
/// asked for on every run, not whatever the previous run happened to leave behind.
pub async fn seed_user(pool: &PgPool, discord_id: &str, username: &str, arma_id: &str, role: &str) {
    let mut tx = pool.begin().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): begin: {e}")
    });

    // Free the arma slot from any *other* discord_id so the upsert cannot unique-violate.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, updated_at = now() \
         WHERE arma_id = $1 AND discord_id IS DISTINCT FROM $2",
    )
    .bind(arma_id)
    .bind(discord_id)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}): release foreign holder: {e}")
    });

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, $2, $2, '', $3, '', $4::user_role, false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, arma_id = EXCLUDED.arma_id, role = EXCLUDED.role, \
          is_banned = false, updated_at = now()",
    )
    .bind(discord_id)
    .bind(username)
    .bind(arma_id)
    .bind(role)
    .execute(&mut *tx)
    .await
    .unwrap_or_else(|e| panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): {e}"));

    tx.commit().await.unwrap_or_else(|e| {
        panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): commit: {e}")
    });
}

/// Mint an access token **without** rewriting the shared `dev-login` row.
///
/// Prefer this whenever the suite needs a specific `discord_id` (private actor) or must not
/// leave `users.role` on `DEV_LOGIN_USER` as `enlisted` for a sibling binary that reads the
/// DB (`misc_integration.rs` asserts `role == admin` via `GET /me`). JWT role gates
/// (`MissionMakerUser`, `AdminUser`, …) read the claim, not the row — so this loses no
/// coverage versus `dev_login_token` for authz paths.
///
/// `suite` appears in the panic so a mint failure names the caller the same way
/// [`dev_login_token`] does.
pub fn access_token(
    state: &AppState,
    suite: &str,
    discord_id: &str,
    role: &str,
    arma_linked: bool,
) -> String {
    state
        .jwt
        .issue_access(discord_id, role, arma_linked)
        .unwrap_or_else(|e| {
            panic!(
                "tests/{suite}.rs: issue_access(discord_id={discord_id}, role={role}, \
                 arma_linked={arma_linked}): {e}"
            )
        })
        .0
}
