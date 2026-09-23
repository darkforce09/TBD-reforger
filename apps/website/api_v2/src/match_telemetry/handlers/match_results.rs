//! `POST /api/v1/ingest/match-results` — the finished-match report: roster validation, the match
//! upsert, per-player facts, attendance attribution, and transactional aggregate recomputation.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use serde_json::{Value, json};

use crate::administration::services::required_audit::append_system_audit;
use crate::command_center::services::leaderboard_view::refresh_leaderboard_on_connection;
use crate::command_center::services::user_stats::recompute_user_stats_on_connection;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::ServiceAuth;
use crate::identity_and_access::services::identity_ownership::{lock_accounts, lock_identities};
use crate::match_telemetry::models::match_record::MissionOutcome;

use super::ingest_parsing::{
    AUDIT_UNLINKED_ID_SAMPLE, parse_uuid_opt_strict, require_role_played, source_match_key,
};
use super::match_results_contract::MatchResultsInput;
use super::match_upsert::upsert_match;
use crate::match_telemetry::services::result_serialization::lock_source_and_prior_identities;
use crate::operations::services::participation_attribution::{
    lock_obligated_registrants, prior_match_accounts, reconcile_match,
};

/// Atomically accept the complete roster and recompute its currently verified account attribution.
/// Unowned identities retain gameplay facts and are reported in the response and required audit.
/// @route POST /api/v1/ingest/match-results
pub async fn ingest_match_results(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<MatchResultsInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("match and players are required"))?;
    let m = input.match_data;

    // `""` is not an accepted outcome: spelled `Pending`, it is how a won match reverts. An
    // unfinished match must say `"pending"` explicitly.
    let outcome = match m.outcome.trim() {
        "success" => MissionOutcome::Success,
        "failure" => MissionOutcome::Failure,
        "aborted" => MissionOutcome::Aborted,
        "pending" => MissionOutcome::Pending,
        _ => return Err(ApiError::bad_request("invalid outcome")),
    };

    // Resolve the dedupe key once, out here with the rest of the match-level validation and
    // before the transaction, so a blank one is a 400 rather than a write (see
    // `source_match_key`). Everything downstream binds *this* value.
    let source_match_id = source_match_key(&m.source_match_id)?;

    // Validate the whole roster before opening the transaction — an empty `arma_id` or
    // `source_event_id` decodes fine but is junk in a row whose dedupe key is
    // `(match_id, arma_id, source_event_id)`, and a blank key silently collapses distinct
    // players onto one row. An empty string is the same lie as a missing field.
    for p in &input.players {
        if p.arma_id.trim().is_empty() || p.arma_id.trim().len() > 128 {
            return Err(ApiError::bad_request(
                "player arma_id must contain 1 to 128 bytes",
            ));
        }
        if p.source_event_id.trim().is_empty() {
            return Err(ApiError::bad_request("player source_event_id is required"));
        }
        // Present-and-blank would otherwise pass and replace a populated role on UPSERT —
        // same blank-reject as `outcome` / `source_match_id` (see `require_role_played`).
        require_role_played(&p.role_played)?;
        // Flat top-level counters are folded at write time via `effective_counters`.
        // Nested `counters` still 400s at decode if the block is present-but-partial.
    }

    let requested_event = parse_uuid_opt_strict("event_id", &m.event_id)?;
    let requested_mission = parse_uuid_opt_strict("mission_id", &m.mission_id)?;
    let mut tx = state.pool.begin().await?;
    let mut all_identities = lock_source_and_prior_identities(&mut tx, source_match_id).await?;
    // Registrants of the old and new exact attachment may gain or lose a no-show obligation.
    let obligated =
        lock_obligated_registrants(&mut tx, source_match_id, requested_event, requested_mission)
            .await?;
    all_identities.extend(input.players.iter().map(|p| p.arma_id.trim().to_owned()));
    let identities: Vec<&str> = all_identities.iter().map(String::as_str).collect();
    lock_identities(&mut tx, &identities).await?;
    let mut affected: Vec<String> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE arma_id = ANY($1) AND deleted_at IS NULL",
    )
    .bind(&identities)
    .fetch_all(&mut *tx)
    .await?;
    let previous: Vec<String> = sqlx::query_scalar("SELECT DISTINCT discord_id FROM match_player_stats WHERE arma_id = ANY($1) AND discord_id IS NOT NULL")
        .bind(&identities).fetch_all(&mut *tx).await?;
    affected.extend(previous);
    affected.extend(prior_match_accounts(&mut tx, source_match_id).await?);
    affected.extend(obligated);
    affected.sort_unstable();
    affected.dedup();
    lock_accounts(&mut tx, &affected).await?;
    let (match_id, _) = upsert_match(&mut tx, &m, outcome, source_match_id).await?;

    let mut resolved: Vec<String> = Vec::new();
    // `unlinked_rows` counts player *lines* with no owner so it sums with the linked count to
    // `players.len()`; `unlinked_ids` is the distinct set, in first-seen order, because one
    // `arma_id` may appear on two lines under different `source_event_id`s and the list exists to
    // name people, not rows.
    let mut unlinked_rows: usize = 0;
    let mut unlinked_ids: Vec<&str> = Vec::new();
    for p in &input.players {
        // Bind the trimmed forms: they are two thirds of the dedupe key, so `"abc"` and `" abc"`
        // must not become two rows for the same player.
        let arma_id = p.arma_id.trim();
        let source_event_id = p.source_event_id.trim();
        // Validated above; re-resolve so the bind is the same trimmed bytes the guard saw.
        let role_played = require_role_played(&p.role_played)?;
        // The one resolver, and the only one there is — nothing else in the crate maps an
        // `arma_id` to an account. `identity_link_codes.arma_id` is written only as a code is
        // consumed, by which point `users.arma_id` is already set, so it holds no answer this
        // query does not. A miss here is therefore final for this request, which is exactly why
        // it has to be reported rather than absorbed.
        let discord_id: Option<String> = sqlx::query_scalar(
            "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
        )
        .bind(arma_id)
        .fetch_optional(&mut *tx)
        .await?;
        match &discord_id {
            Some(did) => {
                if !resolved.contains(did) {
                    resolved.push(did.clone());
                }
            }
            None => {
                unlinked_rows += 1;
                if !unlinked_ids.contains(&arma_id) {
                    unlinked_ids.push(arma_id);
                }
            }
        }
        // `discord_id = EXCLUDED.discord_id` re-asks the resolver on every re-ingest and binds
        // the lookup result verbatim — including NULL. So: a retry after a link claims the
        // row; a retry after an *unlink* (which clears `users.arma_id`) deliberately nulls a
        // previously-populated `discord_id`. That is not a clobber bug: unlink releases the
        // stats rows, and this EXCLUDED write is the ingest-side half of that release.
        // Freezing the first owner would leave orphaned linked rows after unlink. The upsert
        // key includes `arma_id` precisely so this statement and the link-time backfill can
        // find the row again.
        //
        // **Two statements, because "absent counters is not a write" has to be true of the SQL
        // and not just of the struct.** On conflict the counters-absent `DO UPDATE SET` touches
        // only `discord_id` and `role_played`, so a stored scoreline is not read, not rewritten,
        // and not even locked against on those columns.
        //
        // **INSERT half:** omitting the counter columns would materialise DDL `DEFAULT 0` /
        // `false` (`0001_initial_schema.sql:251-265`), and a stored 0 is indistinguishable from a
        // scored 0 — `leaderboard_totals` sums it. Counters are NULLable
        // (`0014_nullable_match_player_stat_counters.sql`); the absent path binds explicit NULLs
        // so a first insert stores "not measured", not zero.
        //
        // Deliberately **not** a read-modify-write (`SELECT` the current counters, re-bind
        // them): that is the same end state through a race. Two concurrent POSTs for one row —
        // a retry overlapping the original, which this endpoint's whole retry contract makes
        // routine — would each read the pre-update values and the later writer would restore
        // the counters the earlier one had just replaced. Not naming a column on UPDATE cannot
        // lose a write that way; re-binding its old value can.
        match p.effective_counters() {
            Some(c) => {
                sqlx::query(
                    "INSERT INTO match_player_stats \
                     (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, \
                      longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
                     ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
                      discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played, \
                      kills = EXCLUDED.kills, deaths = EXCLUDED.deaths, team_kills = EXCLUDED.team_kills, \
                      longest_kill_m = EXCLUDED.longest_kill_m, vehicles_destroyed = EXCLUDED.vehicles_destroyed, \
                      is_command = EXCLUDED.is_command, command_win = EXCLUDED.command_win",
                )
                .bind(match_id)
                .bind(arma_id)
                .bind(&discord_id)
                .bind(role_played)
                .bind(c.kills)
                .bind(c.deaths)
                .bind(c.team_kills)
                .bind(c.longest_kill_m)
                .bind(c.vehicles_destroyed)
                .bind(c.is_command)
                .bind(c.command_win)
                .bind(source_event_id)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO match_player_stats \
                     (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, \
                      longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id) \
                     VALUES ($1, $2, $3, $4, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $5) \
                     ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
                      discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played",
                )
                .bind(match_id)
                .bind(arma_id)
                .bind(&discord_id)
                .bind(role_played)
                .bind(source_event_id)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    reconcile_match(&mut tx, match_id, &affected).await?;
    if !unlinked_ids.is_empty() {
        let shown = unlinked_ids.len().min(AUDIT_UNLINKED_ID_SAMPLE);
        let mut ids = unlinked_ids[..shown].join(", ");
        if unlinked_ids.len() > shown {
            ids.push_str(&format!(", +{} more", unlinked_ids.len() - shown));
        }
        append_system_audit(
            &mut tx,
            "match.unlinked_players",
            "match",
            &match_id.to_string(),
            &format!(
                "{unlinked_rows} of {} player line(s) had no linked account, so their stats are \
                 stored but excluded from the leaderboard and deployment counts until the \
                 identity is linked. Unlinked arma_id(s): {ids}",
                input.players.len()
            ),
        )
        .await?;
    }

    for did in &affected {
        recompute_user_stats_on_connection(&mut tx, did).await?;
    }
    refresh_leaderboard_on_connection(&mut tx).await?;
    tx.commit().await?;

    // `players` is the submitted count and stays that way, because it is the only field a caller
    // may already read (the committed test asserts it, and the stored match model carries nothing
    // from here). What the count alone hides is that it reads as "all of these landed". The three
    // additions are the split, so a sender's own 200 tells it which of its players are invisible
    // to every aggregate: `linked + unlinked == players`, always.
    Ok(Json(json!({
        "match_id": match_id,
        "players": input.players.len(),
        "linked": input.players.len() - unlinked_rows,
        "unlinked": unlinked_rows,
        "unlinked_arma_ids": unlinked_ids,
    })))
}

#[cfg(test)]
#[path = "tests/match_results.rs"]
mod tests;
