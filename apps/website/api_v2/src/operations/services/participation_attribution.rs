//! Match provenance derives attendance independently of reservation and current identity ownership.
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;

/// Include factual signup authors even after an identity moves to another account.
/// Call after source/identity serialization and before acquiring the sorted account lock set.
pub async fn prior_match_accounts(
    connection: &mut PgConnection,
    source_match_id: Option<&str>,
) -> Result<Vec<String>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT registration.discord_id
        FROM event_registration_participation participation
        JOIN event_registrations registration ON registration.id = participation.registration_id
        JOIN matches m ON m.id = participation.match_id WHERE m.source_match_id = $1",
    )
    .bind(source_match_id)
    .fetch_all(connection)
    .await?)
}

/// The caller holds the match's source and identity locks and every affected account lock.
/// Reconciliation retracts invalid match provenance, never the signup or its reservation.
pub async fn reconcile_match(
    connection: &mut PgConnection,
    match_id: Uuid,
    affected_accounts: &[String],
) -> Result<(), ApiError> {
    sqlx::query(
        "DELETE FROM event_registration_participation participation
        USING event_registrations registration, event_missions em
        WHERE participation.match_id = $1 AND registration.id = participation.registration_id
        AND em.id = registration.event_mission_id AND NOT EXISTS (
            SELECT 1 FROM matches m JOIN match_player_stats result ON result.match_id = m.id
            WHERE m.id = participation.match_id AND result.arma_id = participation.arma_id
            AND m.event_id = em.event_id AND m.mission_id = em.mission_id
            AND m.finalized_at IS NOT NULL)",
    )
    .bind(match_id)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "INSERT INTO event_registration_participation (registration_id, match_id, arma_id)
        SELECT DISTINCT registration.id, m.id, result.arma_id
        FROM matches m JOIN match_player_stats result ON result.match_id = m.id
        JOIN event_missions em ON m.event_id = em.event_id AND m.mission_id = em.mission_id
        JOIN event_registrations registration ON registration.event_mission_id = em.id
            AND registration.discord_id = result.discord_id
        WHERE m.id = $1 AND m.finalized_at IS NOT NULL
        ON CONFLICT (registration_id, match_id, arma_id) DO NOTHING",
    )
    .bind(match_id)
    .execute(&mut *connection)
    .await?;
    refresh_attendance(connection, affected_accounts).await
}

/// Attendance is derived by `derived_attendance_state`: finalized participation, a reservation
/// active when its exact match was finalized (no-show), or preserved legacy evidence.
/// The caller holds these accounts' locks. No event or mission parent locks are acquired here.
pub async fn refresh_attendance(
    connection: &mut PgConnection,
    accounts: &[String],
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE event_registrations registration
         SET attendance_state = derived_attendance_state(registration.id)
         WHERE registration.discord_id = ANY($1)
           AND registration.attendance_state IS DISTINCT FROM derived_attendance_state(registration.id)",
    )
    .bind(accounts)
    .execute(connection)
    .await?;
    Ok(())
}

/// Share-lock the exact event attachments a match is, and will be, associated with, in UUID
/// order, and return every registrant of them: finalization can oblige each one to attend.
///
/// Call after the source-match guard and before identity and account locks. Reservation writers
/// take event, then attachment, then account locks, so a telemetry transaction holding only a
/// share lock on the attachment can never wait for them while they wait for it; the share lock
/// keeps registrations of the attachment stable while obligations derive.
pub async fn lock_obligated_registrants(
    connection: &mut PgConnection,
    source_match_id: Option<&str>,
    requested_event: Option<Uuid>,
    requested_mission: Option<Uuid>,
) -> Result<Vec<String>, ApiError> {
    let stored: Option<(Option<Uuid>, Option<Uuid>)> = match source_match_id {
        Some(source) => {
            sqlx::query_as("SELECT event_id, mission_id FROM matches WHERE source_match_id = $1")
                .bind(source)
                .fetch_optional(&mut *connection)
                .await?
        }
        None => None,
    };
    let (stored_event, stored_mission) = stored.unwrap_or((None, None));
    let mut events = vec![requested_event.or(stored_event)];
    let mut missions = vec![requested_mission.or(stored_mission)];
    events.push(stored_event);
    missions.push(stored_mission);
    let attachments: Vec<Uuid> = sqlx::query_scalar(
        "SELECT attachment.id FROM event_missions attachment
         JOIN unnest($1::uuid[], $2::uuid[]) AS association(event_id, mission_id)
           ON association.event_id = attachment.event_id AND association.mission_id = attachment.mission_id
         ORDER BY attachment.id FOR SHARE OF attachment",
    )
    .bind(&events)
    .bind(&missions)
    .fetch_all(&mut *connection)
    .await?;
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT discord_id FROM event_registrations WHERE event_mission_id = ANY($1)",
    )
    .bind(&attachments)
    .fetch_all(connection)
    .await?)
}
