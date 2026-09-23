//! Restore a removed attachment without replacing the historical ORBAT or reopening reservations.
use crate::core::error_handling::api_error::ApiError;
use crate::operations::{models::EventMission, services::OrbatSquadTemplate};
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

type SlotDefinition = (String, String, String, String, String, String, i64);

/// The caller holds the event scope, including removed attachments and their accounts. Different authored data is explicitly rejected so a restore
/// cannot erase historical slot interpretation or silently discard newly submitted fields.
pub async fn restore_if_removed(
    connection: &mut PgConnection,
    event: Uuid,
    mission: Uuid,
    start: DateTime<Utc>,
    template: &[OrbatSquadTemplate],
) -> Result<Option<EventMission>, ApiError> {
    let existing: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, deleted_at IS NOT NULL FROM event_missions
        WHERE event_id = $1 AND mission_id = $2 FOR NO KEY UPDATE",
    )
    .bind(event)
    .bind(mission)
    .fetch_optional(&mut *connection)
    .await?;
    let Some((id, removed)) = existing else {
        return Ok(None);
    };
    if !removed {
        return Err(ApiError::conflict(
            "this mission is already attached to this event",
        ));
    }
    let mut stored: Vec<SlotDefinition> = sqlx::query_as(
        "SELECT faction, squad, COALESCE(callsign, ''), role, COALESCE(loadout, ''), COALESCE(tag, ''), slot_index
        FROM orbat_slots WHERE event_mission_id = $1")
        .bind(id).fetch_all(&mut *connection).await?;
    let mut desired: Vec<SlotDefinition> = template
        .iter()
        .flat_map(|squad| {
            squad.slots.iter().enumerate().map(|(index, slot)| {
                (
                    squad.faction.clone(),
                    squad.squad.clone(),
                    squad.callsign.clone(),
                    slot.role.clone(),
                    slot.loadout.clone(),
                    slot.tag.clone(),
                    index as i64,
                )
            })
        })
        .collect();
    stored.sort();
    desired.sort();
    if stored != desired {
        return Err(ApiError::conflict(
            "restoring a removed mission requires its original ORBAT; use a separate mission for a different seating plan",
        ));
    }
    // The caller's event scope already locked every registrant and occupant of this attachment.
    let accounts: Vec<String> = sqlx::query_scalar(
        "SELECT discord_id FROM event_registrations WHERE event_mission_id = $1
         UNION SELECT assigned_to FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to IS NOT NULL",
    )
    .bind(id)
    .fetch_all(&mut *connection)
    .await?;
    let restored = sqlx::query_as("UPDATE event_missions SET deleted_at = NULL, start_time = $2, updated_at = clock_timestamp()
        WHERE id = $1 RETURNING id, event_id, mission_id, start_time,
        COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at,
        COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at")
        .bind(id).bind(start).fetch_one(&mut *connection).await?;
    for account in accounts {
        crate::command_center::services::user_stats::recompute_user_stats_on_connection(
            connection, &account,
        )
        .await?;
    }
    Ok(Some(restored))
}
