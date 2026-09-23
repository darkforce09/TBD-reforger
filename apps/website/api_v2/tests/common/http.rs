//! Minting persisted integration-test sessions through the router or session service.

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

/// The development administrator identity. Other development roles use distinct accounts.
/// Suites that mutate account state should use dedicated actor IDs.
pub const DEV_LOGIN_USER: &str = "000000000000000001";

/// Mint an access token through `GET /api/v1/auth/dev-login?role={role}`.
///
/// Failure reports identify the calling suite, requested role, HTTP status, body, and redirect.
/// The production session service validates account availability before issuing credentials.
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
            "expected 302 Found. Check the response body: 404 indicates an unavailable \
             development route, 401/403 an unavailable account, and 500 a persistence or \
             session-issuance failure.",
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
pub async fn access_token(
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

    super::fixtures::seed_membership(&state.pool, discord_id, &state.cfg.discord_guild_id, role)
        .await;
    website_api::identity_and_access::services::session_issuance::issue_session(state, discord_id)
        .await
        .unwrap_or_else(|e| {
            panic!("tests/{suite}.rs: issue_session(discord_id={discord_id}, role={role}): {e:?}")
        })
        .0
}
