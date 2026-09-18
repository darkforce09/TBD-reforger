//! The single-use rotating refresh token: `POST /auth/refresh` and `POST /auth/logout`.
//!
//! The rotation invariant is the point of this module. A refresh token may be spent exactly
//! once, and three things all mean the family is compromised: presenting an already-revoked
//! token, losing the conditional `UPDATE` to a concurrent double-spend, and a banned
//! account. Each of those revokes **every** active token for the user, not just the one
//! presented.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::application_state::AppState;
use crate::core::authentication_primitives;
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::go_time;
use crate::identity_and_access::models::user_account::RefreshToken;
use crate::identity_and_access::services::session_issuance::{
    arma_id_is_linked, issue_refresh, revoke_token_family,
};
use crate::identity_and_access::services::user_lookup::load_user;

/// Body for `/auth/refresh` and `/auth/logout`.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    #[serde(default)]
    pub refresh_token: String,
}

/// `POST /api/v1/auth/refresh` — rotate a valid refresh token.
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
    let hash = authentication_primitives::hash_token(&req.refresh_token);

    let rt: Option<RefreshToken> = sqlx::query_as(
        "SELECT id, discord_id, token_hash, expires_at, revoked_at, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
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

    let arma_linked = arma_id_is_linked(&user.arma_id);
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
    let hash = authentication_primitives::hash_token(&req.refresh_token);
    let _: Result<_, _> = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .execute(&state.pool)
    .await;
    Ok(StatusCode::NO_CONTENT)
}
