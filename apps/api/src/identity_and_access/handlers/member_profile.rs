//! The caller's own account: `GET /me` and `PATCH /me`.

use crate::identity_and_access::models::current_profile::{
    CurrentProfileResponse, UpdatedProfileResponse,
};
use axum::extract::State;
use axum::response::Json;
use serde_json::json;

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
) -> Result<Json<CurrentProfileResponse>, ApiError> {
    let Some(mut u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    u.role =
        serde_json::from_value(json!(user.role)).map_err(|_| ApiError::internal("invalid role"))?;
    // Whitespace-only arma_id is not linked — the same rule the JWT mint applies.
    let arma_linked = arma_id_is_linked(&u.arma_id);
    Ok(Json(CurrentProfileResponse {
        user: u,
        arma_linked,
        membership_stale: user.membership_stale,
        membership_override_active: user.membership_override_active,
        can_manage_membership_override: user.can_manage_membership_override,
    }))
}

/// `PATCH /api/v1/me` — answers with the caller's stored user object, unchanged: every profile
/// field is owned by the Discord sign-in or the Arma link flow, so there is nothing a client may
/// set here.
///
/// @route PATCH /api/v1/me
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<UpdatedProfileResponse>, ApiError> {
    let Some(mut u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    u.role =
        serde_json::from_value(json!(user.role)).map_err(|_| ApiError::internal("invalid role"))?;
    Ok(Json(UpdatedProfileResponse { user: u }))
}

#[cfg(test)]
#[path = "tests/member_profile.rs"]
mod tests;
