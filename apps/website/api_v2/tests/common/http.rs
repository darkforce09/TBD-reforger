//! Minting access tokens for integration suites, through the router or the JWT issuer.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;
use website_api::core::application_state::AppState;

/// The shared `dev-login` row's `arma_id`, pinned from `DEV_ARMA_ID` in
/// `src/identity_and_access/handlers/developer_login.rs`.
///
/// `tests/test_support_self_checks.rs` asserts the handler still carries this literal, so a
/// handler-side change turns into a named failure instead of a fixture that quietly drifts
/// out of step with production's row shape.
pub(crate) const DEV_LOGIN_ARMA_ID: &str = "dev-arma-76561190000000001";

/// The single identity `GET /auth/dev-login` mints for **every** role (`DEV_USER_ID` in
/// `src/identity_and_access/handlers/developer_login.rs`). It is shared by every suite that
/// calls dev-login, and each call rewrites that row's `username`, `discord_handle`, `role`
/// and `last_login_at` in the handler's `ON CONFLICT` branch — so a suite must never assume
/// anything about this row beyond what its own most recent dev-login call just wrote.
pub const DEV_LOGIN_USER: &str = "000000000000000001";

/// Mint an access token through `GET /api/v1/auth/dev-login?role={role}`.
///
/// `suite` is the calling file (`"events"`), `role` the tier being requested. Both appear
/// in the failure message, which is the entire point of this helper: on **any** failure it
/// reports the HTTP status, the response body, the URI and who was asking. Reading the
/// `Location` header through `HeaderMap`'s `Index` impl instead — what every hand-rolled copy
/// of this extractor did — panics with `no entry found for key "location"`, naming neither
/// the cause nor the caller, so a 404 (route not registered) and a 500 (database unreachable)
/// are indistinguishable from a code defect.
///
/// # `dev_login` has no ban check
///
/// A missing `Location` is **not** a banned account, however plausible that reads:
///
/// * `src/identity_and_access/handlers/developer_login.rs` — `dev_login` never reads
///   `is_banned`. Its upsert *writes* `is_banned = false` on insert, its `ON CONFLICT` branch
///   does not touch the column, and it then calls `issue_session` unconditionally.
/// * `src/identity_and_access/services/session_issuance.rs` — `issue_session` is
///   `issue_access` + `issue_refresh`. Neither loads the user row, so `is_banned` is never
///   consulted on this path. A banned shared row still gets a **302**.
/// * `src/identity_and_access/handlers/session_tokens.rs` — the ban check lives in `refresh`
///   (`POST /auth/refresh`), which 403s `"account is banned"`. That is a different route.
///
/// The real mechanism behind a missing `Location` is a **shared-fixture collision**: suites
/// mutate the shared `users` row's role, faction links and `arma_id` out from under each
/// other, and dev-login's response is then not what the next suite expects. Give your suite
/// its own actor ids (see [`super::fixtures::seed_user`]) rather than reaching for a ban
/// explanation.
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
             session_redirect (src/identity_and_access/services/session_issuance.rs).",
        );
        panic!("{msg}");
    };
    let Some((_, fragment)) = location.split_once('#') else {
        let msg = ctx.report(
            "Location carries no `#` fragment. auth_callback_url puts the tokens in the \
             fragment (src/identity_and_access/services/session_issuance.rs); a fragment-less \
             Location is an error redirect, and its `error=` query names the reason.",
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

/// Mint an access token **without** rewriting the shared `dev-login` row.
///
/// Prefer this whenever the suite needs a specific `discord_id` (private actor) or must not
/// leave `users.role` on `DEV_LOGIN_USER` as `enlisted` for a sibling binary that reads the
/// DB (`misc_integration.rs` asserts `role == admin` via `GET /me`). JWT role gates
/// (`MissionMakerUser`, `AdminUser`, …) read the claim, not the row — so this loses no
/// coverage versus [`dev_login_token`] for authz paths.
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
