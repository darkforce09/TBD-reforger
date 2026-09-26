//! The writes of one applied results revision: the match row and its player lines.
//!
//! **Role:** merges the revision's match-level fields into the registered match, deletes the
//! removed lines and replaces the present ones.
//! **Position:** called by [`super::match_results_ingest`] inside its transaction, after the match
//! row, identity and account locks.
//! **Signals & state:** none; writes through the caller's connection.
//! **Invariants:** a present match field replaces the stored value and an absent one keeps it;
//! `finalized_at` is set once and never cleared; a present line replaces its role and owner always
//! and its counters when `counters` is present (a higher revision may lower them); a line without
//! `counters` keeps the stored counters and, on first insert, stores NULL ("not measured") rather
//! than a zero the leaderboard would sum; omitted lines are kept.

use uuid::Uuid;

use super::ingest_parsing::{coalesce_str, foreign_key_error, parse_terrain_opt};
use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::match_results_revision::{MatchResultsRevision, PlayerLine};

/// Merge the revision's match-level fields and stamp its number and digest.
pub async fn write_match_row(
    connection: &mut sqlx::PgConnection,
    match_id: Uuid,
    revision: &MatchResultsRevision,
) -> Result<(), ApiError> {
    let report = &revision.report;
    // Validated by the decoder: absent keeps, blank clears, otherwise an absolute http(s) URL.
    let aar_replay_url: Option<&str> = report.aar_replay_url.as_deref().map(str::trim);
    sqlx::query(
        "UPDATE matches SET
            event_id = COALESCE($1, event_id),
            mission_id = COALESCE($2, mission_id),
            terrain = COALESCE($3, terrain),
            started_at = COALESCE($4, started_at),
            ended_at = COALESCE($5, ended_at),
            outcome = $6,
            finalized_at = COALESCE(finalized_at,
                CASE WHEN $6::mission_outcome <> 'pending' THEN clock_timestamp() END),
            winning_faction = COALESCE($7, winning_faction),
            aar_replay_url = COALESCE($8, aar_replay_url),
            revision = $9,
            report_sha256 = $10
         WHERE id = $11",
    )
    .bind(revision.event_id)
    .bind(revision.mission_id)
    .bind(parse_terrain_opt(&report.terrain))
    .bind(report.started_at)
    .bind(report.ended_at)
    .bind(revision.outcome)
    .bind(coalesce_str(&report.winning_faction))
    .bind(aar_replay_url)
    .bind(revision.revision)
    .bind(&revision.report_sha256)
    .bind(match_id)
    .execute(connection)
    .await
    .map_err(|error| foreign_key_error(&error).unwrap_or_else(|| error.into()))?;
    Ok(())
}

/// Delete the rows the revision names in `removed_lines`; naming a missing row is not an error.
pub async fn delete_removed_lines(
    connection: &mut sqlx::PgConnection,
    match_id: Uuid,
    revision: &MatchResultsRevision,
) -> Result<(), ApiError> {
    for line in &revision.removed_lines {
        sqlx::query(
            "DELETE FROM match_player_stats
             WHERE match_id = $1 AND arma_id = $2 AND source_event_id = $3",
        )
        .bind(match_id)
        .bind(line.arma_id.trim())
        .bind(line.source_event_id.trim())
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

/// Insert or replace one present player line owned by `discord_id` (or by nobody).
///
/// Two statements, so "no counters, no counter claim" holds in the SQL: the counters-absent
/// update names only the owner and the role, never re-binding stored counters (a read-modify-write
/// would lose a concurrent writer's values).
pub async fn write_player_line(
    connection: &mut sqlx::PgConnection,
    match_id: Uuid,
    line: &PlayerLine,
    discord_id: Option<&str>,
) -> Result<(), ApiError> {
    let arma_id = line.arma_id.trim();
    let source_event_id = line.source_event_id.trim();
    let role_played = line.role_played.trim();
    match &line.counters {
        Some(counters) => {
            sqlx::query(
                "INSERT INTO match_player_stats
                    (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills,
                     longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET
                    discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played,
                    kills = EXCLUDED.kills, deaths = EXCLUDED.deaths,
                    team_kills = EXCLUDED.team_kills, longest_kill_m = EXCLUDED.longest_kill_m,
                    vehicles_destroyed = EXCLUDED.vehicles_destroyed,
                    is_command = EXCLUDED.is_command, command_win = EXCLUDED.command_win",
            )
            .bind(match_id)
            .bind(arma_id)
            .bind(discord_id)
            .bind(role_played)
            .bind(counters.kills)
            .bind(counters.deaths)
            .bind(counters.team_kills)
            .bind(counters.longest_kill_m)
            .bind(counters.vehicles_destroyed)
            .bind(counters.is_command)
            .bind(counters.command_win)
            .bind(source_event_id)
            .execute(connection)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO match_player_stats
                    (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills,
                     longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id)
                 VALUES ($1, $2, $3, $4, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $5)
                 ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET
                    discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played",
            )
            .bind(match_id)
            .bind(arma_id)
            .bind(discord_id)
            .bind(role_played)
            .bind(source_event_id)
            .execute(connection)
            .await?;
        }
    }
    Ok(())
}
