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

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use tower::ServiceExt;

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
/// `ON CONFLICT DO UPDATE`, not `DO NOTHING`: a suite that owns its ids wants the fixture it
/// asked for on every run, not whatever the previous run happened to leave behind.
pub async fn seed_user(pool: &PgPool, discord_id: &str, username: &str, arma_id: &str, role: &str) {
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
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({discord_id}, arma_id={arma_id}, role={role}): {e}"));
}
