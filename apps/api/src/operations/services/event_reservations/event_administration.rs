//! Event-wide changes serialize with reservations and preserve historical registration facts.
use super::participant_allocations::{load_active_allocations, load_reservation_quotas};
use super::quota_selection::ReservationQuotaUsage;
use super::reservation_release::{release_mission_registrations, release_unused_allocations};
use super::reservation_scope::{AttachmentScope, ReservationScope};
use crate::{
    core::{configuration::Config, error_handling::api_error::ApiError, middleware::AuthUser},
    operations::{
        models::{Event, participant_allocation::ParticipantAllocationKind},
        services::event_status_rules::{EVENT_COLUMNS, sql},
    },
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

/// Event → every attachment by UUID (removed ones included) → complete sorted account union →
/// authoritative administrator session, then legal automatic lifecycle edges are materialized.
pub async fn lock_event_scope(
    connection: &mut PgConnection,
    id: Uuid,
    actor: &AuthUser,
    config: &Config,
) -> Result<ReservationScope, ApiError> {
    let scope = ReservationScope::lock(
        connection,
        id,
        AttachmentScope::IncludingRemoved,
        Some((actor, config)),
        &[],
    )
    .await?;
    if scope.actor()?.role != "admin" {
        return Err(ApiError::forbidden("insufficient role"));
    }
    let stored = sqlx::query_scalar("SELECT status FROM events WHERE id = $1")
        .bind(id)
        .fetch_one(&mut *connection)
        .await?;
    crate::operations::services::event_lifecycle_transition::advance_locked_event(
        connection,
        id,
        stored,
        scope.event.status,
    )
    .await?;
    Ok(scope)
}

/// The parent lock is acquired in a preceding statement, so this projection observes post-wait time.
pub async fn load_locked_event(connection: &mut PgConnection, id: Uuid) -> Result<Event, ApiError> {
    sqlx::query_as(sql(format!(
        "SELECT {} FROM events e WHERE e.id = $1 AND e.deleted_at IS NULL",
        &*EVENT_COLUMNS
    )))
    .bind(id)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| ApiError::not_found("event not found"))
}

/// Zero keeps the established uncapped convention. Every active allocation, and every seat
/// occupant without one, counts once; no pool limit may fall below the places it has granted.
pub async fn validate_capacity(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    maximum: i64,
) -> Result<(), ApiError> {
    if !(0..=256).contains(&maximum) {
        return Err(ApiError::bad_request("max_slots must be between 0 and 256"));
    }
    let usage = allocation_usage(connection, scope.event.id, &scope.active_missions).await?;
    let quotas = load_reservation_quotas(connection, scope.event.id).await?;
    let allocated = usage.total().map_err(ApiError::internal)?;
    if maximum != 0 && allocated > maximum as u64 {
        return Err(ApiError::conflict(format!(
            "max_slots cannot be lower than the {allocated} allocated participants; release reservations first"
        )));
    }
    usage
        .validate_limits(&quotas, maximum as u32)
        .map_err(ApiError::conflict)
}

/// Active allocations by recorded pool; unallocated seat occupants count as unclassified.
pub async fn allocation_usage(
    connection: &mut PgConnection,
    event_id: Uuid,
    active_missions: &[Uuid],
) -> Result<ReservationQuotaUsage, ApiError> {
    let allocations = load_active_allocations(connection, event_id).await?;
    let unallocated_occupants: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT assigned_to) FROM orbat_slots
         WHERE event_mission_id = ANY($1) AND assigned_to IS NOT NULL AND assigned_to <> ALL($2)",
    )
    .bind(active_missions)
    .bind(allocations.keys().cloned().collect::<Vec<_>>())
    .fetch_one(connection)
    .await?;
    let mut usage = ReservationQuotaUsage {
        legacy_unclassified: unallocated_occupants as u64,
        ..Default::default()
    };
    for kind in allocations.values() {
        match kind {
            ParticipantAllocationKind::Member => usage.member += 1,
            ParticipantAllocationKind::Guest => usage.guest += 1,
            ParticipantAllocationKind::Open => usage.open += 1,
            ParticipantAllocationKind::LegacyUnclassified => usage.legacy_unclassified += 1,
        }
    }
    Ok(usage)
}

/// Move active mission times by one exact UTC interval, leaving removed historical attachments alone.
pub async fn reschedule_missions(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    start: DateTime<Utc>,
) -> Result<(), ApiError> {
    let start = normalize_schedule_time(start)?;
    let delta = start.signed_duration_since(scope.event.start_time);
    let microseconds = delta.num_microseconds().ok_or_else(|| {
        ApiError::bad_request("schedule change exceeds supported timestamp range")
    })?;
    let times: Vec<DateTime<Utc>> =
        sqlx::query_scalar("SELECT start_time FROM event_missions WHERE id = ANY($1)")
            .bind(&scope.active_missions)
            .fetch_all(&mut *connection)
            .await?;
    if times.iter().any(|time| {
        time.checked_add_signed(delta)
            .is_none_or(|shifted| normalize_schedule_time(shifted).is_err())
    }) {
        return Err(ApiError::bad_request(
            "a dependent mission time exceeds the supported timestamp range",
        ));
    }
    sqlx::query("UPDATE event_missions SET start_time = start_time + $2, updated_at = clock_timestamp() WHERE id = ANY($1)")
        .bind(&scope.active_missions).bind(sqlx::postgres::types::PgInterval {months: 0, days: 0, microseconds})
        .execute(&mut *connection).await?;
    for account in &scope.accounts {
        crate::command_center::services::user_stats::recompute_user_stats_on_connection(
            connection, account,
        )
        .await?;
    }
    Ok(())
}

/// Delete/cancel releases current reservations and their allocations without erasing signup or
/// attendance observations. No connected player is kicked.
pub async fn release_event_reservations(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    reason: &str,
) -> Result<(), ApiError> {
    let accounts =
        release_mission_registrations(connection, &scope.active_missions, reason).await?;
    release_unused_allocations(connection, scope.event.id, &accounts, reason).await
}

/// The web contract uses four-digit UTC years and PostgreSQL's microsecond timestamp precision.
pub fn normalize_schedule_time(time: DateTime<Utc>) -> Result<DateTime<Utc>, ApiError> {
    if !(1..=9999).contains(&time.year()) || time.nanosecond() >= 1_000_000_000 {
        return Err(ApiError::bad_request(
            "start_time must use a four-digit year without a leap second",
        ));
    }
    Ok(time
        .with_nanosecond(time.nanosecond() / 1000 * 1000)
        .expect("microsecond fraction is valid"))
}
