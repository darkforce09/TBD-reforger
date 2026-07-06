//! Self-service + Arma-link handlers — Rust port of `handlers/me.go`.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth;
use crate::error::ApiError;
use crate::handlers::{is_unique_violation, load_user};
use crate::middleware::{AuthUser, ServiceAuth};
use crate::models::AuditSeverity;
use crate::services;
use crate::state::AppState;

/// 6-digit Arma link-code lifetime (10 minutes).
const LINK_CODE_TTL_MIN: i64 = 10;

/// `GET /api/v1/me` — the caller's user object plus their Arma-link flag.
///
/// @route GET /api/v1/me
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    let arma_linked = u.arma_id.is_some();
    Ok(Json(json!({ "user": u, "arma_linked": arma_linked })))
}

/// `PATCH /api/v1/me` — placeholder echo (profile fields come from Discord/link flow).
///
/// @route PATCH /api/v1/me
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    Ok(Json(json!({ "user": u })))
}

/// `POST /api/v1/me/link` — issue a fresh 6-digit link code (201), invalidating the
/// caller's prior unconsumed codes.
///
/// @route POST /api/v1/me/link
pub async fn create_link_code(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Expire previous outstanding codes so only the newest is valid (best-effort).
    let _ = sqlx::query(
        "UPDATE identity_link_codes SET expires_at = now() \
         WHERE discord_id = $1 AND consumed_at IS NULL",
    )
    .bind(&user.discord_id)
    .execute(&state.pool)
    .await;

    // Generate a unique code (retry on the rare PK collision).
    for _ in 0..5 {
        let code = auth::numeric_code(6);
        let expires = Utc::now() + Duration::minutes(LINK_CODE_TTL_MIN);
        let res = sqlx::query(
            "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
             VALUES ($1, $2, $3, now())",
        )
        .bind(&code)
        .bind(&user.discord_id)
        .bind(expires)
        .execute(&state.pool)
        .await;
        match res {
            Ok(_) => {
                return Ok((
                    StatusCode::CREATED,
                    Json(json!({
                        "code": code,
                        "expires_at": crate::models::serde_helpers::go_time::format(&expires),
                    })),
                ));
            }
            Err(e) if is_unique_violation(&e) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(ApiError::internal("could not allocate code"))
}

/// `GET /api/v1/me/link/status` — link + pending-code state for UI polling.
///
/// @route GET /api/v1/me/link/status
pub async fn link_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_link_codes \
         WHERE discord_id = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "linked": u.arma_id.is_some(),
        "arma_id": u.arma_id,
        "arma_character": u.arma_character,
        "pending_code": pending > 0,
    })))
}

/// `DELETE /api/v1/me/link` — remove the Arma association.
///
/// @route DELETE /api/v1/me/link
pub async fn unlink(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    // arma_character is a non-null string column (app never writes NULL) → set '' not
    // NULL; wire output ("") is identical to Go, and reads still decode into String.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, arma_character = '', updated_at = now() \
         WHERE discord_id = $1",
    )
    .bind(&user.discord_id)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "linked": false })))
}

/// Body posted by the in-game mod (service-token authenticated).
#[derive(Debug, Deserialize)]
pub struct LinkConfirmRequest {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub arma_id: String,
    #[serde(default)]
    pub arma_character: String,
}

/// `POST /api/v1/ingest/link-confirm` — consume a pending code + attach the Arma id.
///
/// @route POST /api/v1/ingest/link-confirm
pub async fn ingest_link_confirm(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<LinkConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) = body.map_err(|_| ApiError::bad_request("code and arma_id required"))?;
    if req.code.is_empty() || req.arma_id.is_empty() {
        return Err(ApiError::bad_request("code and arma_id required"));
    }

    // Look up a live (unconsumed, unexpired) code.
    let found: Option<(String, String)> = sqlx::query_as(
        "SELECT code, discord_id FROM identity_link_codes \
         WHERE code = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(&req.code)
    .fetch_optional(&state.pool)
    .await?;
    let Some((code, discord_id)) = found else {
        return Err(ApiError::not_found("invalid or expired code"));
    };

    // Guard against linking an Arma ID already owned by another account.
    let clash: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE arma_id = $1 AND discord_id <> $2")
            .bind(&req.arma_id)
            .bind(&discord_id)
            .fetch_one(&state.pool)
            .await?;
    if clash > 0 {
        return Err(ApiError::conflict(
            "arma id already linked to another account",
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE identity_link_codes SET consumed_at = now(), arma_id = $1 WHERE code = $2")
        .bind(&req.arma_id)
        .bind(&code)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE users SET arma_id = $1, arma_character = $2, updated_at = now() \
         WHERE discord_id = $3",
    )
    .bind(&req.arma_id)
    .bind(&req.arma_character)
    .bind(&discord_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal("could not link identity"))?;
    tx.commit()
        .await
        .map_err(|_| ApiError::internal("could not link identity"))?;

    // Best-effort audit (username reload; failure must not fail the request).
    let username = load_user(&state.pool, &discord_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();
    services::write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(&discord_id),
        &username,
        "identity.link",
        &format!("{username} successfully linked their Arma Steam ID"),
        "user",
        &discord_id,
    )
    .await;

    Ok(Json(json!({
        "linked": true,
        "discord_id": discord_id,
        "arma_id": req.arma_id,
        "arma_character": req.arma_character,
    })))
}
