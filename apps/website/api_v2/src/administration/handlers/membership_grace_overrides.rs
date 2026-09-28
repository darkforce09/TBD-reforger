//! Administrative recovery of cached permissions during Discord outages.
//!
//! @contract personnel-actions.schema.json#/definitions/MembershipGraceRequest
//! @contract personnel-actions.schema.json#/definitions/MembershipGraceExtension

use crate::core::{
    application_state::AppState, error_handling::api_error::ApiError,
    http::path_parameters::PathParams, middleware::AuthUser,
};
use crate::identity_and_access::services::membership_grace_overrides::extend_membership_grace;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipGraceInput {
    pub duration_hours: i64,
    pub reason: String,
}

/// @route POST /api/v1/admin/users/:discordId/membership-grace
pub async fn extend_grace(
    State(state): State<AppState>,
    actor: AuthUser,
    PathParams(discord_id): PathParams<String>,
    body: Result<Json<MembershipGraceInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;
    let expires_at = extend_membership_grace(
        &state,
        &actor,
        &discord_id,
        input.duration_hours,
        &input.reason,
    )
    .await?;
    Ok(Json(
        json!({"discord_id": discord_id, "expires_at": expires_at}),
    ))
}
