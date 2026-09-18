//! The caller's own account: `GET /me` and `PATCH /me`.

use axum::extract::State;
use axum::response::Json;
use serde_json::{Value, json};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::identity_and_access::services::session_issuance::arma_id_is_linked;
use crate::identity_and_access::services::user_lookup::load_user;

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
    // Whitespace-only arma_id is not linked — the same rule the JWT mint applies.
    let arma_linked = arma_id_is_linked(&u.arma_id);
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

#[cfg(test)]
#[path = "tests/member_profile.rs"]
mod tests;
