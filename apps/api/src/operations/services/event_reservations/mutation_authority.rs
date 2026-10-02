//! Authority checks reservation writers apply after acquiring the event scope's locks.
use crate::{
    core::{
        error_handling::api_error::ApiError,
        middleware::{AuthUser, role_rank},
    },
    operations::models::event::{Event, OrbatSlot},
};
use sqlx::PgConnection;
use uuid::Uuid;

pub fn require_leader(actor: &AuthUser) -> Result<(), ApiError> {
    if role_rank(&actor.role) < role_rank("leader") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    Ok(())
}

/// A squad hold is mutable authority and must be read under its event parent lock.
pub async fn require_squad_management(
    connection: &mut PgConnection,
    actor: &AuthUser,
    mission: Uuid,
    squad: &str,
) -> Result<(), ApiError> {
    require_leader(actor)?;
    if actor.role == "admin" {
        return Ok(());
    }
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(mission)
    .bind(squad)
    .fetch_optional(connection)
    .await?;
    if owner.as_deref() != Some(&actor.discord_id) {
        return Err(ApiError::forbidden(
            "reserve this squad to manage its slots",
        ));
    }
    Ok(())
}

pub async fn load_slot(
    connection: &mut PgConnection,
    mission: Uuid,
    slot: Uuid,
) -> Result<OrbatSlot, ApiError> {
    sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
        .bind(slot).bind(mission).fetch_optional(connection).await?
        .ok_or_else(|| ApiError::not_found("slot not found"))
}

/// Registration status is read from the event loaded after the parent lock wait. Only
/// administrators may act while registration is locked.
pub fn require_registration_open(event: &Event, is_admin: bool) -> Result<(), ApiError> {
    if !crate::operations::services::event_status_rules::can_register_status(event.status) {
        return Err(ApiError::conflict(
            "registration is closed for this operation",
        ));
    }
    if event.registration_locked && !is_admin {
        return Err(ApiError::forbidden(
            "registration is locked; an admin must assign you",
        ));
    }
    Ok(())
}
