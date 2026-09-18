//! `POST /api/v1/ingest/match-results` — the finished-match report: roster validation, the match
//! upsert, the per-player stat rows, attendance attribution, and the post-commit recompute.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::command_center::services::user_stats::{
    recompute_user_stats, refresh_leaderboard_best_effort,
};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::ServiceAuth;
use crate::match_telemetry::models::match_record::MissionOutcome;

use super::attendance_attribution::{mark_attended, retract_prior_attendance};
use super::ingest_parsing::{AUDIT_UNLINKED_ID_SAMPLE, require_role_played, source_match_key};
use super::match_results_contract::MatchResultsInput;
use super::match_upsert::upsert_match;

/// `POST /api/v1/ingest/match-results` — idempotent match + per-player stats,
/// attendance marking, user-stat recompute, leaderboard refresh (service-token).
///
/// **A player whose `arma_id` resolves to no account keeps their row, and the 200 says so out
/// loud.** The row was never the problem; the *silence* was. `discord_id` on
/// `match_player_stats` is a cached answer to "who owns this `arma_id`" (see
/// `identity_and_access::handlers::arma_link_confirmation`'s `BACKFILL_MATCH_STATS`),
/// `leaderboard_totals` filters `WHERE discord_id IS NOT NULL` (`0001_initial_schema.sql:289`)
/// and `recompute_user_stats` counts only non-NULL rows — so an unresolved row is invisible to
/// every aggregate on the platform. Reporting only `{"players": n}`, the *submitted* count, hides
/// that. Measured on a throwaway database: one POST carrying `kills=17 deaths=3
/// longest_kill_m=842 vehicles_destroyed=4` for an unlinked `arma_id` returns
/// `{"match_id":"…","players":1}`, writes the row with `discord_id` NULL, and leaves
/// `leaderboard_totals` with **zero** rows for that player and `users.total_deployments` at
/// **0**. Nothing anywhere records that a scoreline has gone missing.
///
/// **Three fixes were on the table and only one of them is honest:**
///
/// * **400 the POST** — rejected, for three reasons of increasing force. (1) The row is *real
///   telemetry*: the `arma_id` is real, the match happened, the counters were measured, and the
///   row is *recoverable* — the link-confirm handler claims exactly the `discord_id IS NULL` rows
///   at link time, so parking it loses a player from the aggregates while rejecting it loses the
///   data. (2) There is no per-player 400 to be had: the roster is validated before the
///   transaction and the transaction is atomic, so one unresolved player would reject the **whole
///   op** — the match row and every other player's line with it. (3) Decisively, it is not a
///   sender error at all. `users.arma_id` is written by exactly two things, the dev seed and
///   `POST /ingest/link-confirm`, and the shipping mod **does not implement the link flow** —
///   `TBD_ResultsReporter.c:23-35` says so in its own header ("in production no player has an
///   `arma_id` … There is no `#tbd link` command"). An unresolved `arma_id` is not the edge case
///   today; it is *every player in every production match*. A 400 would reject 100% of live
///   ingest to report a condition the platform is currently always in.
/// * **Drop the row instead of storing it NULL** — rejected outright, and named only because it
///   is the reading of "stop losing rows" that would make the loss permanent. It would also break
///   the retroactive link: with no row to claim, linking would backfill nothing.
/// * **Keep the row, keep the 200, and end the silence** — taken. Nothing stored changes. The
///   response stops implying the roster landed (`linked` / `unlinked` / `unlinked_arma_ids`
///   beside the unchanged `players`), and one audit row per affected ingest names the count and
///   the ids, so the drop is discoverable by an operator and not only by whoever is reading the
///   game server's console.
///
/// **The audit row is `Info`, not `Warn`, and that is the whole judgement rather than a
/// default.** An unresolved `arma_id` is a normal, expected, self-healing state — the player
/// simply has not linked yet, and the link is retroactive. A `Warn` would fire on every single
/// production ingest, and a warning that is always on is a warning nobody reads. `Info` records
/// the fact at the severity the fact actually has.
///
/// Two consequences a reader will ask about, both intended. A **retry** appends a second audit
/// row: the log records requests, retries must stay legal, and "we were told this twice" is true.
/// And `linked + unlinked == players` **always**, because those two count player *lines* —
/// `unlinked_arma_ids` is the distinct set, since the same `arma_id` may legitimately appear
/// twice under different `source_event_id`s and an operator chasing links wants each person once.
///
/// What this does **not** do is widen `leaderboard_totals` or `recompute_user_stats` to include
/// unowned rows. Both are per-account aggregates and an unowned row has no account to aggregate
/// onto; the fix for its absence is the link, not a leaderboard entry with no one on the other
/// end of it.
///
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
        if p.arma_id.trim().is_empty() {
            return Err(ApiError::bad_request("player arma_id is required"));
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

    let mut tx = state.pool.begin().await?;
    // `event_id` from the upsert is not the attendance key: the UPDATE below joins the *merged*
    // match row so both `event_id` and `mission_id` must be present.
    //
    // `retract_from`: when a re-POST moves this match off a fully attributed
    // `(event_id, mission_id)` pair, attendance on that prior event_mission must be undone
    // for these players (unless another match still attributes them there).
    let (match_id, retract_from) = upsert_match(&mut tx, &m, outcome, source_match_id).await?;

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

    // Attendance, inside the same transaction and in this order: retract the pair this match has
    // moved off before marking the pair it now names, so a re-point cannot leave both attended.
    // See `attendance_attribution` for each statement's attribution guard.
    if !resolved.is_empty() {
        if let Some((old_event, old_mission)) = retract_from {
            retract_prior_attendance(&mut tx, &resolved, old_event, old_mission, match_id).await?;
        }
        mark_attended(&mut tx, match_id, &resolved).await?;
    }
    tx.commit().await?;

    // The drop goes on the record. Deliberately the FIRST thing after the commit:
    // `recompute_user_stats` below propagates with `?`, so an audit written after it would be
    // skipped by exactly the failure that most needs a trace. Post-commit rather than inside the
    // transaction because it must describe what actually landed, and best-effort (`write_audit`
    // returns `()`) because a missing audit row must not fail an ingest that succeeded.
    if !unlinked_ids.is_empty() {
        let shown = unlinked_ids.len().min(AUDIT_UNLINKED_ID_SAMPLE);
        let mut ids = unlinked_ids[..shown].join(", ");
        if unlinked_ids.len() > shown {
            ids.push_str(&format!(", +{} more", unlinked_ids.len() - shown));
        }
        write_audit(
            &state.pool,
            AuditSeverity::Info,
            None,
            "system",
            "match.unlinked_players",
            &format!(
                "{unlinked_rows} of {} player line(s) had no linked account, so their stats are \
                 stored but excluded from the leaderboard and deployment counts until the \
                 identity is linked. Unlinked arma_id(s): {ids}",
                input.players.len()
            ),
            "match",
            &match_id.to_string(),
        )
        .await;
    }

    // Recompute denormalized user stats + refresh the leaderboard view.
    for did in &resolved {
        recompute_user_stats(&state.pool, did).await?;
    }
    refresh_leaderboard_best_effort(
        &state.pool,
        "Leaderboard refresh failed after match ingest",
        "match",
        &match_id.to_string(),
    )
    .await;

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
