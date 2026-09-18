//! Web-role administration: setting a single member's role, and re-applying the Discord
//! role-mapping table across the whole roster.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::identity_and_access::models::user_account::UserRole;
use crate::identity_and_access::services::discord_role_sync::resync_all_roles;

fn valid_role(s: &str) -> Option<UserRole> {
    match s {
        "enlisted" => Some(UserRole::Enlisted),
        "leader" => Some(UserRole::Leader),
        "mission_maker" => Some(UserRole::MissionMaker),
        "admin" => Some(UserRole::Admin),
        _ => None,
    }
}

/// The role-change body.
///
/// **`role` is deliberately required — do not add `#[serde(default)]` to it.** A default is not
/// absence: it decodes as an affirmative empty string, and this field sits one guard away from a
/// write that sets a privilege level, which is the shape that turns a defaulted request into a
/// silent demotion. The `map_err` below already returns the identical 400 with the identical
/// message for a missing field, so requiring it is invisible on the wire.
#[derive(Debug, Deserialize)]
pub struct UpdateUserInput {
    role: String,
}

/// `PATCH /api/v1/admin/users/:discordId` — set a user's web role.
///
/// @route PATCH /api/v1/admin/users/:discordId
pub async fn update_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<UpdateUserInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("role required"))?;
    // This `is_empty()` is deliberately NOT `trim().is_empty()`, and `role` is deliberately not
    // trimmed before `valid_role`. This guard fails *closed*: `valid_role` is an exact match over
    // four literals, so `"  admin  "` is rejected with 400 "invalid role" and the value bound
    // into the `UPDATE` is the `UserRole` enum, never the request string. There is therefore no
    // untrimmed write here and no reachable bad state — only a debatable error message. Trimming
    // would *loosen* what the endpoint accepts, which is a product decision rather than a bug
    // fix, so it is not taken unilaterally. Counter-precedent worth knowing if that call is ever
    // revisited: `handlers/telemetry/telemetry.rs` matches on a trimmed `outcome` before its enum
    // match, i.e. the crate is not unanimous on this.
    if input.role.is_empty() {
        return Err(ApiError::bad_request("role required"));
    }
    let Some(role) = valid_role(&input.role) else {
        return Err(ApiError::bad_request("invalid role"));
    };
    let res = sqlx::query("UPDATE users SET role = $1 WHERE discord_id = $2")
        .bind(role)
        .bind(&discord_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    let target_name = actor_display_name(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "user.role_change",
        &format!("{actor_name} set {target_name} role to {}", role.as_str()),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "discord_id": discord_id, "role": role })))
}

/// `POST /api/v1/admin/roles/sync` — re-apply discord_roles mappings.
///
/// @route POST /api/v1/admin/roles/sync
pub async fn resync_roles(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<Value>, ApiError> {
    let updated = resync_all_roles(&state.pool)
        .await
        .map_err(|_| ApiError::internal("resync failed"))?;
    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "roles.resync",
        &format!("{actor_name} triggered a role resync"),
        "system",
        "",
    )
    .await;
    Ok(Json(json!({ "updated": updated })))
}
