//! Auth handlers — Rust port of the token half of `handlers/auth.go`: the
//! single-use rotating refresh token (`refresh`/`logout`) and the session helpers
//! shared with dev-login + the Discord callback. The rotation invariant (gate G7a)
//! is preserved verbatim: reuse of a revoked token, a lost double-spend race, or a
//! banned account all revoke the whole token family.

use axum::body::Body;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{StatusCode, header};
use axum::response::{Json, Response};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::auth;
use crate::error::ApiError;
use crate::handlers::load_user;
use crate::models::RefreshToken;
use crate::models::serde_helpers::go_time;
use crate::state::AppState;

/// Opaque refresh token lifetime (30 days).
const REFRESH_TTL_DAYS: i64 = 30;

/// Body for `/auth/refresh` and `/auth/logout`.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    #[serde(default)]
    pub refresh_token: String,
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
    let raw = auth::random_token(32);
    let hash = auth::hash_token(&raw);
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
/// strings so tokens aren't logged upstream). Mirrors `authCallbackURL`.
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

/// A 302 Found redirect to `location` (Go uses `http.StatusFound` for SPA callbacks).
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

/// `POST /api/v1/auth/refresh` — rotate a valid refresh token (gate G7a).
///
/// @route POST /api/v1/auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    body: Result<Json<RefreshRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) = body.map_err(|_| ApiError::bad_request("refresh_token required"))?;
    if req.refresh_token.is_empty() {
        return Err(ApiError::bad_request("refresh_token required"));
    }
    let hash = auth::hash_token(&req.refresh_token);

    let rt: Option<RefreshToken> = sqlx::query_as(
        "SELECT id, discord_id, token_hash, expires_at, revoked_at, created_at \
         FROM refresh_tokens WHERE token_hash = $1",
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await?;
    let Some(rt) = rt else {
        return Err(ApiError::unauthorized("invalid refresh token"));
    };

    // Presenting an already-revoked token is a reuse signal → revoke the whole family.
    if rt.revoked_at.is_some() {
        revoke_token_family(&state.pool, &rt.discord_id).await;
        return Err(ApiError::unauthorized("refresh token reuse detected"));
    }
    if Utc::now() > rt.expires_at {
        return Err(ApiError::unauthorized("expired refresh token"));
    }

    let Some(user) = load_user(&state.pool, &rt.discord_id).await? else {
        return Err(ApiError::unauthorized("user not found"));
    };
    if user.is_banned {
        revoke_token_family(&state.pool, &rt.discord_id).await;
        return Err(ApiError::forbidden("account is banned"));
    }

    // Rotate atomically: only the request that flips revoked_at wins. A concurrent
    // double-spend loses the conditional UPDATE and is treated as reuse.
    let res = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(rt.id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() != 1 {
        revoke_token_family(&state.pool, &rt.discord_id).await;
        return Err(ApiError::unauthorized("refresh token reuse detected"));
    }

    let arma_linked = user.arma_id.is_some();
    let (access, exp) = state
        .jwt
        .issue_access(&user.discord_id, user.role.as_str(), arma_linked)
        .map_err(|_| ApiError::internal("could not issue token"))?;
    let new_refresh = issue_refresh(&state.pool, &user.discord_id)
        .await
        .map_err(|_| ApiError::internal("could not issue refresh token"))?;

    Ok(Json(json!({
        "access_token": access,
        "expires_at": go_time::format(&exp),
        "refresh_token": new_refresh,
        "token_type": "Bearer",
    })))
}

/// `POST /api/v1/auth/logout` — revoke the presented token. Always 204 (no leak).
///
/// @route POST /api/v1/auth/logout
pub async fn logout(
    State(state): State<AppState>,
    body: Result<Json<RefreshRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(req) = body.map_err(|_| ApiError::bad_request("refresh_token required"))?;
    if req.refresh_token.is_empty() {
        return Err(ApiError::bad_request("refresh_token required"));
    }
    let hash = auth::hash_token(&req.refresh_token);
    let _: Result<_, _> = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .execute(&state.pool)
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Shared by dev-login + the Discord callback: build the token-fragment redirect.
/// The `expires_at` in the fragment uses RFC3339 **seconds** (Go `time.RFC3339`),
/// distinct from the nanos form in the refresh JSON body.
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
