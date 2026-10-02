//! Releasing a reservation keeps its identity, signup facts, attendance and history. An already
//! withdrawn registration keeps the reason it was first released with; releasing again only
//! clears a seat it may still name. The caller holds the event scope and the account lock.

use sqlx::PgConnection;
use uuid::Uuid;

use super::participant_allocations::release_allocation_if_unused;
use crate::core::error_handling::api_error::ApiError;

/// Stable release reasons recorded on the registration and its history.
pub mod release_reasons {
    pub const PARTICIPANT_WITHDREW: &str = "participant_withdrew";
    pub const MISSION_REMOVED: &str = "mission_removed";
    pub const EVENT_CANCELLED: &str = "event_cancelled";
    pub const EVENT_DELETED: &str = "event_deleted";
    pub const ELIGIBILITY_LOST: &str = "eligibility_lost";
    pub const ACCESS_POLICY_CHANGED: &str = "access_policy_changed";
    pub const ACCOUNT_UNAVAILABLE: &str = "account_unavailable";
    pub const SEAT_CLEARED: &str = "seat_cleared";
}

/// Withdraw one registration. Returns whether the stored row changed.
pub async fn release_registration(
    connection: &mut PgConnection,
    registration: Uuid,
    reason: &str,
) -> Result<bool, ApiError> {
    Ok(sqlx::query(
        "UPDATE event_registrations SET reservation_state = 'withdrawn', slot_id = NULL,
             allocation_id = NULL, withdrawn_at = COALESCE(withdrawn_at, clock_timestamp()),
             release_reason = CASE WHEN reservation_state = 'withdrawn' THEN release_reason ELSE $2 END
         WHERE id = $1 AND (reservation_state <> 'withdrawn' OR slot_id IS NOT NULL)",
    )
    .bind(registration)
    .bind(reason)
    .execute(connection)
    .await?
    .rows_affected()
        == 1)
}

/// Withdraw every current reservation of the attachments. Returns the affected accounts.
pub async fn release_mission_registrations(
    connection: &mut PgConnection,
    missions: &[Uuid],
    reason: &str,
) -> Result<Vec<String>, ApiError> {
    let mut accounts: Vec<String> = sqlx::query_scalar(
        "UPDATE event_registrations SET reservation_state = 'withdrawn', slot_id = NULL,
             allocation_id = NULL, withdrawn_at = COALESCE(withdrawn_at, clock_timestamp()),
             release_reason = CASE WHEN reservation_state = 'withdrawn' THEN release_reason ELSE $2 END
         WHERE event_mission_id = ANY($1) AND (reservation_state <> 'withdrawn' OR slot_id IS NOT NULL)
         RETURNING discord_id",
    )
    .bind(missions)
    .bind(reason)
    .fetch_all(&mut *connection)
    .await?;
    let occupants: Vec<String> = sqlx::query_scalar(
        "WITH previous AS (
             SELECT id, assigned_to FROM orbat_slots
             WHERE event_mission_id = ANY($1) AND assigned_to IS NOT NULL)
         UPDATE orbat_slots seat SET assigned_to = NULL, assigned_at = NULL
         FROM previous WHERE seat.id = previous.id
         RETURNING previous.assigned_to",
    )
    .bind(missions)
    .fetch_all(&mut *connection)
    .await?;
    accounts.extend(occupants);
    accounts.sort_unstable();
    accounts.dedup();
    Ok(accounts)
}

/// Clear one seat. A registration naming it keeps its place as a seatless holder.
/// Returns the previous occupant, if any.
pub async fn clear_seat(
    connection: &mut PgConnection,
    mission: Uuid,
    seat: Uuid,
) -> Result<(Option<String>, u64), ApiError> {
    let occupant: Option<Option<String>> = sqlx::query_scalar(
        "WITH previous AS (
             SELECT id, assigned_to FROM orbat_slots
             WHERE id = $1 AND event_mission_id = $2 AND assigned_to IS NOT NULL)
         UPDATE orbat_slots seat SET assigned_to = NULL, assigned_at = NULL
         FROM previous WHERE seat.id = previous.id
         RETURNING previous.assigned_to",
    )
    .bind(seat)
    .bind(mission)
    .fetch_optional(&mut *connection)
    .await?;
    let repaired = sqlx::query(
        "UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2",
    )
    .bind(mission)
    .bind(seat)
    .execute(connection)
    .await?
    .rows_affected();
    Ok((occupant.flatten(), repaired))
}

/// Release the allocations of accounts left with no reservation or seat in the event.
pub async fn release_unused_allocations(
    connection: &mut PgConnection,
    event_id: Uuid,
    accounts: &[String],
    reason: &str,
) -> Result<(), ApiError> {
    for account in accounts {
        release_allocation_if_unused(connection, event_id, account, reason).await?;
    }
    Ok(())
}
