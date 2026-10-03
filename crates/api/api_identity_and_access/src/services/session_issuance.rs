//! Session minting and the SPA callback redirect.
//!
//! Every way of signing in — the Discord callback and the development login shortcut —
//! ends here: an access JWT plus a single-use opaque refresh token, handed to the SPA in
//! a URL fragment so credentials do not enter proxy query logs. Persistence and signing
//! succeed in the same account-serialized transaction.

use api_identifiers::DiscordUserId;
use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use api_foundation::error_handling::api_error::ApiError;
use api_state::AppState;

use super::session_storage::{create_session, insert_refresh};
use api_caller_identity::UserRole;
use api_caller_identity::account_authority::{load_account_authority, lock_account};
use api_caller_identity::arma_identity_link::arma_id_is_linked;

/// Mint a session from current authoritative permissions.
pub async fn issue_session(
    state: &AppState,
    discord_id: &DiscordUserId,
) -> Result<(String, DateTime<Utc>, String), ApiError> {
    issue_session_with_development_role(state, discord_id, None).await
}

/// Explicit development credentials are unusable by a production-configured API.
pub async fn issue_development_session(
    state: &AppState,
    discord_id: &DiscordUserId,
    role: UserRole,
) -> Result<(String, DateTime<Utc>, String), ApiError> {
    if !state.cfg.is_development() {
        return Err(ApiError::not_found("not found"));
    }
    issue_session_with_development_role(state, discord_id, Some(role)).await
}

async fn issue_session_with_development_role(
    state: &AppState,
    discord_id: &DiscordUserId,
    development_role: Option<UserRole>,
) -> Result<(String, DateTime<Utc>, String), ApiError> {
    let mut tx = state.pool.begin().await?;
    lock_account(&mut tx, discord_id).await?;
    let account = load_account_authority(&mut tx, discord_id, &state.cfg.discord_guild_id)
        .await?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;
    let role = account
        .permissions(account.observed_at)
        .effective_role
        .ok_or_else(|| ApiError::forbidden("account is banned"))?;
    let session_id = create_session(&mut tx, discord_id, development_role).await?;
    let refresh = insert_refresh(&mut tx, discord_id, session_id).await?;
    let (access, exp) = state
        .jwt
        .issue_access(
            discord_id,
            session_id,
            development_role.unwrap_or(role).as_str(),
            arma_id_is_linked(&account.arma_id),
        )
        .map_err(|_| ApiError::internal("could not issue token"))?;
    api_audit_log::required_audit::append_required_audit(
        &mut tx,
        discord_id,
        "auth.session_created",
        discord_id,
        "Authentication session created",
    )
    .await?;
    tx.commit().await?;
    Ok((access, exp, refresh))
}

/// Issue a persisted refresh-only session; permissions are checked when it is redeemed.
pub async fn issue_refresh(pool: &PgPool, discord_id: &DiscordUserId) -> Result<String, ApiError> {
    let mut tx = pool.begin().await?;
    lock_account(&mut tx, discord_id).await?;
    let available: bool = sqlx::query_scalar(
        "SELECT NOT is_banned AND deleted_at IS NULL FROM users WHERE discord_id = $1",
    )
    .bind(discord_id)
    .fetch_one(&mut *tx)
    .await?;
    if !available {
        return Err(ApiError::unauthorized("account unavailable"));
    }
    let session_id = create_session(&mut tx, discord_id, None).await?;
    let refresh = insert_refresh(&mut tx, discord_id, session_id).await?;
    tx.commit().await?;
    Ok(refresh)
}

/// Build the SPA callback URL with values in the URL fragment (kept out of query
/// strings so tokens aren't logged upstream).
pub fn auth_callback_url(frontend_url: &str, pairs: &[(&str, &str)]) -> String {
    let mut sorted: Vec<&(&str, &str)> = pairs.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in sorted {
        ser.append_pair(k, v);
    }
    format!(
        "{}/auth/callback#{}",
        frontend_url.trim_end_matches('/'),
        ser.finish()
    )
}

/// A 302 Found redirect to `location` — the status every SPA callback redirect uses.
pub fn found(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, location)
        .body(Body::empty())
        .expect("redirect response")
}

/// Redirect the browser back to the SPA callback with an error code.
pub fn redirect_auth_error(frontend_url: &str, reason: &str) -> Response {
    found(&auth_callback_url(frontend_url, &[("error", reason)]))
}

/// Shared by the development login and the Discord callback: build the token-fragment
/// redirect. The `expires_at` in the fragment uses RFC3339 **seconds**, distinct from the
/// nanosecond form the refresh JSON body carries.
pub fn session_redirect(
    frontend_url: &str,
    access: &str,
    refresh: &str,
    exp: DateTime<Utc>,
    arma_linked: bool,
) -> Response {
    let exp_secs = exp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let arma = if arma_linked { "true" } else { "false" };
    found(&auth_callback_url(
        frontend_url,
        &[
            ("access_token", access),
            ("refresh_token", refresh),
            ("expires_at", &exp_secs),
            ("arma_linked", arma),
        ],
    ))
}
