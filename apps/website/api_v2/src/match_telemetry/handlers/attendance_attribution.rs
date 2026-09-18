//! Attendance attribution for a finished match: retract the prior event_mission when a re-ingest
//! moves the match off it, then mark the played event_mission attended.
//!
//! Both statements run on the caller's open ingest transaction — they take the transaction's
//! `PgConnection` so they cannot be reordered relative to the per-player upserts that precede
//! them or escape the atomicity that makes the retract safe.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;

/// Undo attendance on the event_mission this match has just been moved *off*.
///
/// Re-pointing a match from one event to another marks the new pair attended but leaves the old
/// one attended too, so `attendance_rate` inflates to 100% with two past registrations both
/// `attended`. There is no `match_id` on `event_registrations` (storing provenance would require
/// a migration), so the write is made reversible by attributing through live match rows: retract
/// the prior pair for these players only when no *other* match still points at that pair with a
/// linked `match_player_stats` row for them. Restoring `registered` (rather than inventing
/// `waitlisted`) matches the normal path into `attended`.
pub(super) async fn retract_prior_attendance(
    conn: &mut PgConnection,
    resolved: &[String],
    old_event: Uuid,
    old_mission: Uuid,
    match_id: Uuid,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE event_registrations er SET state = 'registered' \
         WHERE er.discord_id = ANY($1) \
           AND er.state::text = 'attended' \
           AND er.event_mission_id IN ( \
             SELECT em.id FROM event_missions em \
             WHERE em.event_id = $2 AND em.mission_id = $3 \
           ) \
           AND NOT EXISTS ( \
             SELECT 1 FROM matches m \
             INNER JOIN match_player_stats mps \
               ON mps.match_id = m.id AND mps.discord_id = er.discord_id \
             WHERE m.id <> $4 \
               AND m.event_id = $2 \
               AND m.mission_id = $3 \
           )",
    )
    .bind(resolved)
    .bind(old_event)
    .bind(old_mission)
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Mark attendance for the *played* event_mission only.
///
/// Keying on the event alone —
/// `WHERE event_mission_id IN (SELECT id FROM event_missions WHERE event_id = $1)` — flips every
/// registration on the event, including missions that were never played. Measured side effects:
/// `decorate_events` and the dashboard count only `registered`/`waitlisted`, so a completed op's
/// roster collapses to zero; withdraw also stops promoting the waitlist once state is `attended`
/// (`was_registered` is false).
///
/// Scoped through the match row's `(event_id, mission_id)` pair — unique on `event_missions`
/// (`idx_event_mission`). Both columns must be set on the match: an event-only ingest cannot know
/// which mission was played, and "mark them all" is the bug this closes. The JOIN reads the
/// *merged* match after `upsert_match`'s COALESCE, so a corrected re-POST that lands `mission_id`
/// still marks attendance.
pub(super) async fn mark_attended(
    conn: &mut PgConnection,
    match_id: Uuid,
    resolved: &[String],
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE event_registrations SET state = 'attended' \
         WHERE discord_id = ANY($2) \
           AND event_mission_id IN ( \
             SELECT em.id FROM event_missions em \
             INNER JOIN matches m \
               ON m.event_id = em.event_id AND m.mission_id = em.mission_id \
             WHERE m.id = $1 \
               AND m.event_id IS NOT NULL \
               AND m.mission_id IS NOT NULL \
           )",
    )
    .bind(match_id)
    .bind(resolved)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
