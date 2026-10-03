//! Single-use refresh rotation and transactional logout. Replaying a consumed token in an active
//! session revokes the account's current sessions. Already retired sessions cannot revoke later
//! recovery logins. Account locking serializes replay, successor issuance and revocation.
//!
//! A body that does not decode answers through [`ApiError::from_json_rejection`] (413 over the
//! body limit, 415 without a JSON content type, 400 otherwise); an empty `refresh_token` answers
//! 400.
//!
//! @contract session-token.schema.json#/definitions/SessionTokenRequest
//! @contract session-token.schema.json#/definitions/SessionTokenPair

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::services::session_rotation::{logout_session, rotate_session};
use api_foundation::error_handling::api_error::ApiError;
use api_state::AppState;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Body for `/auth/refresh` and `/auth/logout`.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /// The opaque refresh token the session was issued with; empty when absent.
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
    let Json(req) = body.map_err(ApiError::from_json_rejection)?;
    if req.refresh_token.is_empty() {
        return Err(ApiError::bad_request("refresh_token required"));
    }
    let (access, exp, new_refresh) = rotate_session(&state, &req.refresh_token).await?;

    Ok(Json(json!({
        "access_token": access,
        "expires_at": rfc3339_utc::format(&exp),
        "refresh_token": new_refresh,
        "token_type": "Bearer",
    })))
}

/// `POST /api/v1/auth/logout` — revoke the session. Unknown tokens return 204; storage failures are errors.
///
/// @route POST /api/v1/auth/logout
pub async fn logout(
    State(state): State<AppState>,
    body: Result<Json<RefreshRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(req) = body.map_err(ApiError::from_json_rejection)?;
    if req.refresh_token.is_empty() {
        return Err(ApiError::bad_request("refresh_token required"));
    }
    logout_session(&state, &req.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}
