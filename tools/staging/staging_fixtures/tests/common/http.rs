//! Bearers for the API under test, and one bearer-authenticated request.
//!
//! **Role:** mints access tokens through the dev-login route or straight from the session
//! service, and sends one request with a bearer.
//! **Position:** called by the `staging_fixtures` suites that check, through the API, that what
//! the tool seeded serves real requests; the router is [`super::api_under_test`]'s.
//! **Signals & state:** none; every session it mints is persisted in the suite's database.
//! **Invariants:** a dev-login that mints no session panics with the suite, the role, the status,
//! the body and the redirect; [`access_token`] seeds the account and its verified membership
//! before it issues the session.

use api_state::AppState;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

/// The shared `dev-login` row's `arma_id`, pinned from `DEV_ARMA_ID` in
/// `crates/api/api_identity_and_access/src/handlers/developer_login.rs`.
///
/// The API's `crates/api/api_server/tests/test_support_self_checks.rs` asserts the handler still carries this
/// literal for the API's own copy of this constant, so a handler-side change turns into a named
/// failure there before this fixture can drift out of step with production's row shape.
pub(crate) const DEV_LOGIN_ARMA_ID: &str = "dev-arma-76561190000000001";

/// The development administrator identity. Other development roles use distinct accounts.
/// Suites that mutate account state should use dedicated actor IDs.
pub(crate) const DEV_LOGIN_USER: &str = "000000000000000001";

/// Mint an access token through `GET /api/v1/auth/dev-login?role={role}`.
///
/// Failure reports identify the calling suite, requested role, HTTP status, body, and redirect.
/// The production session service validates account availability before issuing credentials.
pub(crate) async fn dev_login_token(app: &Router, suite: &str, role: &str) -> String {
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
            "expected 302 Found. Check the response body: 404 indicates an unavailable \
             development route, 401/403 an unavailable account, and 500 a persistence or \
             session-issuance failure.",
        );
        panic!("{msg}");
    }
    let Some(location) = ctx.location else {
        let msg = ctx.report(
            "302 with no Location header. The redirect was built by something other than \
             session_redirect (crates/api/api_identity_and_access/src/services/session_issuance.rs).",
        );
        panic!("{msg}");
    };
    let Some((_, fragment)) = location.split_once('#') else {
        let msg = ctx.report(
            "Location carries no `#` fragment. auth_callback_url puts the tokens in the \
             fragment (crates/api/api_identity_and_access/src/services/session_issuance.rs); a fragment-less \
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
             actor:    development role={role}\n  \
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

/// Seed explicit verified membership and issue a persisted session for a suite-owned account.
///
/// Existing Arma identity and ban state remain intact. `arma_linked` supplies the initial identity
/// only when the account does not exist; token claims are derived from the persisted account.
pub(crate) async fn access_token(
    state: &AppState,
    suite: &str,
    discord_id: &str,
    role: &str,
    arma_linked: bool,
) -> String {
    let initial_arma_id = arma_linked.then(|| format!("test-arma:{discord_id}"));
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Integration Test', 'integration-test', '', $2, '', $3::user_role, \
         false, '', now(), now()) ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(discord_id)
    .bind(initial_arma_id)
    .bind(role)
    .execute(&state.pool)
    .await
    .unwrap_or_else(|error| panic!("tests/{suite}.rs: create actor {discord_id}: {error}"));

    super::fixtures::seed_membership(
        &state.pool,
        discord_id,
        state.cfg.discord_guild_id.as_str(),
        role,
    )
    .await;
    api_identity_and_access::services::session_issuance::issue_session(
        state,
        &api_identifiers::DiscordUserId::new(discord_id),
    )
    .await
    .unwrap_or_else(|e| {
        panic!("tests/{suite}.rs: issue_session(discord_id={discord_id}, role={role}): {e:?}")
    })
    .0
}

/// One bearer-authenticated request against the router: the answer's status and its JSON body
/// (`Value::Null` when the body is not JSON).
pub(crate) async fn call(
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
        .expect("build the request");
    let resp = app.clone().oneshot(req).await.expect("the router answers");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read the answer body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
