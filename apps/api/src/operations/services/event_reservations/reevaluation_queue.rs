//! Durable requests to re-evaluate events whose eligibility inputs changed outside their scope.
//!
//! Producers upsert a row while holding only account locks, so they never take event locks and
//! cannot invert the reservation lock order. A worker leases a due row in a short transaction,
//! then re-evaluates the event in a separate event-first transaction. A request that changes
//! while leased keeps its row, so its newer change is re-evaluated in another pass.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;

/// Request a pass for `event_id` at `due_at`, or immediately. An earlier due time wins.
pub async fn request_reevaluation(
    connection: &mut PgConnection,
    event_id: Uuid,
    due_at: Option<DateTime<Utc>>,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO event_reservation_reevaluations (event_id, due_at)
         VALUES ($1, COALESCE($2, clock_timestamp()))
         ON CONFLICT (event_id) DO UPDATE SET
             due_at = LEAST(event_reservation_reevaluations.due_at, EXCLUDED.due_at),
             revision = event_reservation_reevaluations.revision + 1,
             requested_at = clock_timestamp()",
    )
    .bind(event_id)
    .bind(due_at)
    .execute(connection)
    .await?;
    Ok(())
}

/// Every non-terminal event in which the account reserves, waits, or occupies a seat.
/// The caller holds the account lock of the change being reported.
pub async fn request_reevaluation_for_account(
    connection: &mut PgConnection,
    discord_id: &str,
) -> Result<u64, ApiError> {
    Ok(sqlx::query(
        "INSERT INTO event_reservation_reevaluations (event_id, due_at)
         SELECT DISTINCT mission.event_id, clock_timestamp() FROM event_missions mission
         JOIN events event_row ON event_row.id = mission.event_id
         WHERE mission.deleted_at IS NULL AND event_row.deleted_at IS NULL
           AND event_row.status NOT IN ('completed', 'cancelled')
           AND (EXISTS (SELECT 1 FROM event_registrations registration
                    WHERE registration.event_mission_id = mission.id AND registration.discord_id = $1
                      AND registration.reservation_state IN ('registered', 'legacy_unknown', 'waitlisted'))
                OR EXISTS (SELECT 1 FROM orbat_slots slot
                    WHERE slot.event_mission_id = mission.id AND slot.assigned_to = $1))
         ON CONFLICT (event_id) DO UPDATE SET
             due_at = LEAST(event_reservation_reevaluations.due_at, EXCLUDED.due_at),
             revision = event_reservation_reevaluations.revision + 1,
             requested_at = clock_timestamp()",
    )
    .bind(discord_id)
    .execute(connection)
    .await?
    .rows_affected())
}

/// Schedule a pass at each pool's future opening time; the earliest one is kept.
pub async fn schedule_pool_openings(
    connection: &mut PgConnection,
    event_id: Uuid,
) -> Result<(), ApiError> {
    let next: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT min(opens_at) FROM event_reservation_quota_pools
         WHERE event_id = $1 AND opens_at > clock_timestamp()",
    )
    .bind(event_id)
    .fetch_one(&mut *connection)
    .await?;
    if let Some(opens_at) = next {
        request_reevaluation(connection, event_id, Some(opens_at)).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct ReevaluationLease {
    pub event_id: Uuid,
    pub revision: i64,
    pub lease_token: Uuid,
}

/// Lease the earliest due request that no live worker holds.
pub async fn claim_due_reevaluation(pool: &PgPool) -> Result<Option<ReevaluationLease>, ApiError> {
    Ok(sqlx::query_as(
        "UPDATE event_reservation_reevaluations SET lease_token = $1,
             lease_expires_at = clock_timestamp() + interval '60 seconds', attempts = attempts + 1
         WHERE event_id = (
             SELECT event_id FROM event_reservation_reevaluations
             WHERE due_at <= clock_timestamp()
               AND (lease_expires_at IS NULL OR lease_expires_at <= clock_timestamp())
             ORDER BY due_at, event_id LIMIT 1 FOR UPDATE SKIP LOCKED)
         RETURNING event_id, revision, lease_token",
    )
    .bind(Uuid::new_v4())
    .fetch_optional(pool)
    .await?)
}

/// Finish a pass inside its business transaction. A request renewed meanwhile stays queued.
pub async fn complete_reevaluation(
    connection: &mut PgConnection,
    lease: &ReevaluationLease,
) -> Result<(), ApiError> {
    let deleted = sqlx::query(
        "DELETE FROM event_reservation_reevaluations
         WHERE event_id = $1 AND revision = $2 AND lease_token = $3",
    )
    .bind(lease.event_id)
    .bind(lease.revision)
    .bind(lease.lease_token)
    .execute(&mut *connection)
    .await?
    .rows_affected();
    if deleted == 0 {
        sqlx::query(
            "UPDATE event_reservation_reevaluations SET lease_token = NULL, lease_expires_at = NULL,
                 attempts = 0, last_error = NULL
             WHERE event_id = $1 AND lease_token = $2",
        )
        .bind(lease.event_id)
        .bind(lease.lease_token)
        .execute(connection)
        .await?;
    }
    Ok(())
}

/// Record a failed pass and retry with bounded exponential backoff.
pub async fn fail_reevaluation(
    pool: &PgPool,
    lease: &ReevaluationLease,
    error: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE event_reservation_reevaluations SET lease_token = NULL, lease_expires_at = NULL,
             last_error = left($3, 512),
             due_at = clock_timestamp() + LEAST(interval '10 minutes', interval '2 seconds' * power(2, LEAST(attempts, 9)))
         WHERE event_id = $1 AND lease_token = $2",
    )
    .bind(lease.event_id)
    .bind(lease.lease_token)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}
