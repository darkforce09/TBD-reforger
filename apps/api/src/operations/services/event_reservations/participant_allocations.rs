//! Persisted quota allocations. One active allocation per participant and event records the pool
//! that participant consumed; every active mission reservation of the participant references it.
//! Callers hold the event scope and the participant's account lock.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::reservation_planning::QuotaSettings;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::participant_allocation::ParticipantAllocationKind;
use crate::operations::models::reservation_quota::{
    ReservationQuotaKind, ReservationQuotaPool, ReservationQuotas,
};

/// The event's three pools; a missing or out-of-range row is a storage defect, never a default.
pub async fn load_reservation_quotas(
    connection: &mut PgConnection,
    event_id: Uuid,
) -> Result<ReservationQuotas, ApiError> {
    let rows: Vec<(String, Option<i64>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT quota_kind, seat_limit, opens_at FROM event_reservation_quota_pools WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_all(connection)
    .await?;
    let pool = |kind: ReservationQuotaKind| -> Result<ReservationQuotaPool, ApiError> {
        let (_, limit, opens_at) = rows
            .iter()
            .find(|(stored, _, _)| stored == kind.as_str())
            .ok_or_else(|| ApiError::internal("event reservation pools are incomplete"))?;
        let seats = limit
            .map(u32::try_from)
            .transpose()
            .map_err(|_| ApiError::internal("stored reservation pool limit is out of range"))?;
        Ok(ReservationQuotaPool {
            seats,
            opens_at: *opens_at,
        })
    };
    Ok(ReservationQuotas {
        member: pool(ReservationQuotaKind::Member)?,
        guest: pool(ReservationQuotaKind::Guest)?,
        open: pool(ReservationQuotaKind::Open)?,
    })
}

/// Pools, the event-wide limit and the database clock, read after the scope's lock wait.
pub async fn load_quota_settings(
    connection: &mut PgConnection,
    event_id: Uuid,
    max_slots: i64,
) -> Result<QuotaSettings, ApiError> {
    let quotas = load_reservation_quotas(connection, event_id).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(connection)
        .await?;
    let max_slots = u32::try_from(max_slots)
        .map_err(|_| ApiError::internal("stored event capacity is out of range"))?;
    Ok(QuotaSettings {
        quotas,
        max_slots,
        now,
    })
}

pub async fn load_active_allocations(
    connection: &mut PgConnection,
    event_id: Uuid,
) -> Result<BTreeMap<String, ParticipantAllocationKind>, ApiError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT discord_id, quota_kind FROM event_participant_allocations
         WHERE event_id = $1 AND released_at IS NULL",
    )
    .bind(event_id)
    .fetch_all(connection)
    .await?;
    rows.into_iter()
        .map(|(account, kind)| {
            ParticipantAllocationKind::parse(&kind)
                .map(|kind| (account, kind))
                .ok_or_else(|| ApiError::internal("stored allocation kind is unknown"))
        })
        .collect()
}

/// The participant's active allocation, if any.
pub async fn active_allocation(
    connection: &mut PgConnection,
    event_id: Uuid,
    account: &str,
) -> Result<Option<Uuid>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM event_participant_allocations
         WHERE event_id = $1 AND discord_id = $2 AND released_at IS NULL",
    )
    .bind(event_id)
    .bind(account)
    .fetch_optional(connection)
    .await?)
}

/// Reuse the participant's allocation, or record one. `kind` names the pool a new participant
/// consumes; `None` means the plan found the participant already holding a place, which for a
/// seat occupied without a recorded allocation is recorded as an unclassified historical place.
pub async fn ensure_allocation(
    connection: &mut PgConnection,
    event_id: Uuid,
    account: &str,
    kind: Option<ReservationQuotaKind>,
) -> Result<Uuid, ApiError> {
    if let Some(existing) = active_allocation(connection, event_id, account).await? {
        return Ok(existing);
    }
    let recorded = match kind {
        Some(kind) => ParticipantAllocationKind::from(kind),
        None => ParticipantAllocationKind::LegacyUnclassified,
    };
    Ok(sqlx::query_scalar(
        "INSERT INTO event_participant_allocations (event_id, discord_id, quota_kind)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(event_id)
    .bind(account)
    .bind(recorded.as_str())
    .fetch_one(connection)
    .await?)
}

/// Release the participant's allocation once no active reservation or seat remains in the event.
pub async fn release_allocation_if_unused(
    connection: &mut PgConnection,
    event_id: Uuid,
    account: &str,
    reason: &str,
) -> Result<bool, ApiError> {
    Ok(sqlx::query(
        "UPDATE event_participant_allocations allocation
         SET released_at = clock_timestamp(), release_reason = $3
         WHERE allocation.event_id = $1 AND allocation.discord_id = $2 AND allocation.released_at IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM event_registrations registration
               JOIN event_missions mission ON mission.id = registration.event_mission_id
               WHERE mission.event_id = $1 AND registration.discord_id = $2
                 AND registration.reservation_state IN ('registered', 'legacy_unknown'))
           AND NOT EXISTS (
               SELECT 1 FROM orbat_slots slot JOIN event_missions mission ON mission.id = slot.event_mission_id
               WHERE mission.event_id = $1 AND slot.assigned_to = $2)",
    )
    .bind(event_id)
    .bind(account)
    .bind(reason)
    .execute(connection)
    .await?
    .rows_affected()
        == 1)
}
