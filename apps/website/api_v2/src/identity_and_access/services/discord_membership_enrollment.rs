//! Enrolls guild snapshots that event eligibility needs verified. Enrollment only asks the
//! bot-authenticated REST reconciler to observe an account; it never asserts membership.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;

/// Enroll `guild` for every current participant and waiting entrant of the event, and ask for a
/// prompt observation. The caller holds the event's administrator scope.
pub async fn enroll_event_partner_guild(
    connection: &mut PgConnection,
    event_id: Uuid,
    guild_id: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        "WITH participants AS (
             SELECT DISTINCT registration.discord_id FROM event_registrations registration
             JOIN event_missions mission ON mission.id = registration.event_mission_id
             JOIN users account ON account.discord_id = registration.discord_id
             WHERE mission.event_id = $1 AND mission.deleted_at IS NULL AND account.deleted_at IS NULL
               AND registration.reservation_state IN ('registered', 'legacy_unknown', 'waitlisted'))
         INSERT INTO discord_membership_snapshots (discord_id, guild_id)
         SELECT discord_id, $2 FROM participants
         ON CONFLICT (discord_id, guild_id) DO UPDATE
             SET next_refresh_at = LEAST(discord_membership_snapshots.next_refresh_at, clock_timestamp())",
    )
    .bind(event_id)
    .bind(guild_id)
    .execute(connection)
    .await?;
    Ok(())
}

/// A participant was refused pending verification: enroll every guild the event's policies can
/// rely on for that account and request a prompt observation of each.
pub async fn request_event_membership_verification(
    pool: &PgPool,
    event_id: Uuid,
    discord_id: &str,
    main_guild: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        "WITH guilds AS (
             SELECT $3::text AS guild_id WHERE $3 <> ''
             UNION SELECT source->>'guild_id' FROM event_groups
             WHERE event_id = $1 AND deleted_at IS NULL AND source->>'kind' = 'partner_guild')
         INSERT INTO discord_membership_snapshots (discord_id, guild_id)
         SELECT $2, guild_id FROM guilds WHERE guild_id IS NOT NULL
             AND EXISTS (SELECT 1 FROM users WHERE discord_id = $2 AND deleted_at IS NULL)
         ON CONFLICT (discord_id, guild_id) DO UPDATE
             SET next_refresh_at = LEAST(discord_membership_snapshots.next_refresh_at, clock_timestamp())",
    )
    .bind(event_id)
    .bind(discord_id)
    .bind(main_guild)
    .execute(pool)
    .await?;
    Ok(())
}
