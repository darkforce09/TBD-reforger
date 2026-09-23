//! Account linking code issuance, status, and explicit unlinking.
use crate::core::{
    application_state::AppState, error_handling::api_error::ApiError, middleware::AuthUser,
};
use crate::identity_and_access::services::{
    identity_linking::unlink_identity, link_code_issuance::issue_link_code,
    session_issuance::arma_id_is_linked, user_lookup::load_user,
};
use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::{Value, json};

/// Issue a single-use code and cancel this account's previous pending code atomically.
/// @route POST /api/v1/me/link
pub async fn create_link_code(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (code, expires) = issue_link_code(&state, &user).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"code": code,
        "expires_at": crate::core::wire_format::rfc3339_utc::format(&expires)})),
    ))
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
         WHERE discord_id = $1 AND consumed_at IS NULL AND cancelled_at IS NULL AND expires_at > clock_timestamp()",
    )
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "linked": arma_id_is_linked(&u.arma_id),
        "arma_id": u.arma_id,
        "arma_character": u.arma_character,
        "pending_code": pending > 0,
    })))
}

/// Remove current identity ownership and pending codes while preserving signup and attendance facts.
/// @route DELETE /api/v1/me/link
pub async fn unlink(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    unlink_identity(&state, &user).await?;
    Ok(Json(json!({ "linked": false })))
}
