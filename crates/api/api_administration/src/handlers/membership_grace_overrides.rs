//! Administrative recovery of cached permissions during Discord outages.
//!
//! @contract personnel-actions.schema.json#/definitions/MembershipGraceRequest
//! @contract personnel-actions.schema.json#/definitions/MembershipGraceExtension

use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AuthUser;
use api_identifiers::DiscordUserId;
use api_identity_and_access::services::membership_grace_overrides::extend_membership_grace;
use api_state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use serde::Deserialize;
use serde_json::{Value, json};

/// The body of a membership grace extension.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipGraceInput {
    /// How long the cached permissions are extended, in hours (the service accepts 1 to 48).
    pub duration_hours: i64,
    /// Why the administrator extends the grace, recorded in the audit line.
    pub reason: String,
}

/// @route POST /api/v1/admin/users/:discordId/membership-grace
pub async fn extend_grace(
    State(state): State<AppState>,
    actor: AuthUser,
    PathParams(discord_id): PathParams<DiscordUserId>,
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
