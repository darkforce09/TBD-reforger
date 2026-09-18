//! Session minting and the SPA callback redirect.
//!
//! Every way of signing in — the Discord callback and the development login shortcut —
//! ends here: an access JWT plus a single-use opaque refresh token, handed to the SPA in
//! a URL fragment so the credentials never enter a query string that an upstream proxy
//! would log. [`revoke_token_family`] is the response to a detected refresh-token reuse
//! and to a banned account: the whole family dies, not just the presented token.

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

use crate::core::application_state::AppState;
use crate::core::authentication_primitives;
use crate::core::error_handling::api_error::ApiError;

/// Opaque refresh token lifetime (30 days).
const REFRESH_TTL_DAYS: i64 = 30;

/// True when `arma_id` is present **and** non-whitespace after trim.
///
/// Read-side only — does not mutate storage. A row can hold `Some("   ")`, and
/// `Option::is_some()` alone reports that as linked: the JWT claim and the SPA gates then
/// say LINKED while every resolve path, which binds the trimmed id, finds nothing.
///
/// Trimming is safe *here*: the flag is derived for the claim, never written back, and it
/// takes no part in the byte-identity joins (`orbat_reservations.squad` / `or_fallback`)
/// where a one-sided trim would change which rows match.
pub fn arma_id_is_linked(arma_id: &Option<String>) -> bool {
    arma_id.as_deref().is_some_and(|s| !s.trim().is_empty())
}

/// Mint a fresh access + refresh pair for a user.
pub async fn issue_session(
    state: &AppState,
    discord_id: &str,
    role: &str,
    arma_linked: bool,
) -> Result<(String, DateTime<Utc>, String), ApiError> {
    let (access, exp) = state
        .jwt
        .issue_access(discord_id, role, arma_linked)
        .map_err(|_| ApiError::internal("could not issue token"))?;
    let refresh = issue_refresh(&state.pool, discord_id).await?;
    Ok((access, exp, refresh))
}

/// Create + store a new opaque refresh token (hashed); return the raw value.
pub async fn issue_refresh(pool: &PgPool, discord_id: &str) -> Result<String, ApiError> {
    let raw = authentication_primitives::random_token(32);
    let hash = authentication_primitives::hash_token(&raw);
    let expires_at = Utc::now() + Duration::days(REFRESH_TTL_DAYS);
    sqlx::query(
        "INSERT INTO refresh_tokens (discord_id, token_hash, expires_at, created_at) \
         VALUES ($1, $2, $3, now())",
    )
    .bind(discord_id)
    .bind(&hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(raw)
}

/// Revoke every active refresh token for a user — the response to detected reuse or
/// a banned account. Best-effort: a failure is logged but the caller's 401/403 stands.
pub async fn revoke_token_family(pool: &PgPool, discord_id: &str) {
    if let Err(e) = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE discord_id = $1 AND revoked_at IS NULL",
    )
    .bind(discord_id)
    .execute(pool)
    .await
    {
        tracing::error!(error = %e, discord_id, "token family revocation failed");
    }
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

#[cfg(test)]
#[path = "tests/session_issuance.rs"]
mod tests;
