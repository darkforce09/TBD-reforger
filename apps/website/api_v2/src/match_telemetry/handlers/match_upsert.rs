//! The idempotent write of the `matches` row: find by `source_match_id` and merge, or create.

use chrono::Utc;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::core::text::http_url_guard::is_http_url;
use crate::match_telemetry::models::match_record::MissionOutcome;

use super::ingest_parsing::{
    coalesce_str, foreign_key_error, parse_terrain_opt, parse_uuid_opt_strict,
};
use super::match_results_contract::MatchInput;

/// Find a match by source_match_id (updating mutable fields) or create one. Returns
/// `(id, retract_from)`.
///
/// `retract_from` is `Some((event_id, mission_id))` when this was a re-ingest that *moved* the
/// match off a previously fully attributed event_mission pair. The caller uses it to undo
/// attendance that would otherwise stick on the old pair. Create and no-op merges return `None`.
///
/// `source_match_id` arrives **already normalized** from `source_match_key` and is the only form
/// this function may use — it deliberately does not read `MatchInput::source_match_id`, because a
/// lookup and an INSERT reading two different forms of the same field is the whole defect that
/// normalization closes.
pub(super) async fn upsert_match(
    tx: &mut sqlx::PgConnection,
    m: &MatchInput,
    outcome: MissionOutcome,
    source_match_id: Option<&str>,
) -> Result<(Uuid, Option<(Uuid, Uuid)>), ApiError> {
    let event_id = parse_uuid_opt_strict("event_id", &m.event_id)?;
    let mission_id = parse_uuid_opt_strict("mission_id", &m.mission_id)?;
    // Terrain stays optional. Known pins (`everon`/`arland`/`custom`) map to the enum;
    // anything else soft-fails to NULL — see `parse_terrain_opt`. Rejecting instead would 400
    // the whole report for community terrain names that the mission schema otherwise allows.
    let terrain = parse_terrain_opt(&m.terrain);

    // **The live XSS this guard closes.** The SPA binds this column straight into an `<a href>`
    // and nothing on the way in looked at it, so `javascript:alert(1)` stored cleanly and executed
    // on click. The `rel="noreferrer"` already on that anchor is not a mitigation: it governs the
    // `Referer` header, not what the scheme does. Neither is HTML escaping — a `javascript:` href
    // is not a quote breakout, it is a well-formed attribute whose *content* runs.
    //
    // Validated **here** rather than in `ingest_match_results` because this function is the write
    // boundary: it owns both statements that can put a value in the column, so a second caller
    // added later cannot route around the check by construction.
    //
    // Rejected rather than stored-and-escaped-on-read — see `is_http_url` for why. Three input
    // shapes, and the middle one is the reason this is not a one-liner:
    //
    // - **absent** → `None` → `COALESCE` keeps what is already stored. That is the keep rule for
    //   this field: the replay is uploaded *after* the match, so the POST carrying the result
    //   usually cannot name the link yet.
    // - **blank** → `Some("")` → clears the link (`COALESCE('', col)` yields `''`, because `''`
    //   is not NULL). Preserved deliberately: an empty string carries no scheme and cannot
    //   execute, so 400-ing it would break a working shape to buy nothing. This is deliberately
    //   *not* `source_match_id`'s "blank is neither, so it is a 400" — that field is a lookup key,
    //   where a blank silently collapses distinct matches onto one row; this one is a nullable
    //   display value, where blank is the honest way to say "no replay".
    // - **anything else** → must be an `http`/`https` URL. Trimmed first, same as `terrain` above,
    //   so the bytes validated are the bytes stored.
    //
    // The 400 does not echo the offending value back. The sender is a game server whose only
    // channel is this string (`TBD_ResultsReporter.c` `OnSendError` logs the response body
    // verbatim), and reflecting an attacker-chosen payload into that log buys nobody anything —
    // the sender already knows what it sent.
    let aar_replay_url: Option<&str> = match m.aar_replay_url.as_deref().map(str::trim) {
        None => None,
        Some("") => Some(""),
        Some(u) if is_http_url(u) => Some(u),
        Some(_) => {
            return Err(ApiError::bad_request(
                "aar_replay_url must be an absolute http:// or https:// URL",
            ));
        }
    };

    if let Some(src) = source_match_id {
        let existing: Option<(Uuid, Option<Uuid>, Option<Uuid>, bool)> = sqlx::query_as(
            "SELECT id, event_id, mission_id, finalized_at IS NOT NULL FROM matches WHERE source_match_id = $1",
        )
        .bind(src)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id, prior_event, prior_mission, finalized)) = existing {
            if finalized && outcome == MissionOutcome::Pending {
                return Err(ApiError::conflict(
                    "a finalized match cannot return to pending",
                ));
            }
            // `COALESCE($n, <column>)` — a bare column name on the right of a SET reads the
            // pre-update row, so an omitted field keeps what is already there instead of
            // overwriting it with a decoded default. `outcome` is bound unconditionally
            // because it is required on the way in.
            //
            // **All seven optional fields read the same way, and the four leading ones have to
            // be in this statement.** Left out of it, a *present* value is silently discarded
            // rather than applied — the exact opposite of the rule their siblings follow ("only
            // the overwrite was ever the bug").
            //
            // The consequence is not cosmetic. A first POST carrying a `source_match_id` but no
            // `event_id`, then a corrected re-POST carrying the right one, marks **nobody's**
            // attendance, forever, on two 200s — measured: `event_id` and `mission_id` still
            // NULL, `terrain` still NULL, `started_at` still the first POST's `now()`, and
            // `event_registrations.state` still `registered` with `attendance_rate` 0.0.
            // Silently absorbing a correction on an endpoint with no human in the loop is the
            // same objection that rules out `GREATEST` for the counters.
            //
            // `started_at` binds `m.started_at`, **never** the create path's
            // `unwrap_or_else(Utc::now)` — that default is a value the sender did not send, so
            // COALESCE-ing it here would stamp every partial retry with the retry's own clock.
            // The default is therefore computed at the INSERT and cannot reach this statement.
            //
            // `RETURNING event_id, mission_id` returns the **merged** pair: reading a pre-update
            // column instead is the other half of why a correction does nothing. Attendance
            // itself joins the match row rather than binding this return, but the COALESCE
            // write that lands `mission_id` here is what that JOIN reads — so the merge is
            // still load-bearing, just one statement later. Comparing the merged pair to the
            // pre-update pair yields `retract_from` (only when the prior pair was fully
            // attributed — the SET path never marks on a half-null match).
            let merged: (Option<Uuid>, Option<Uuid>) = sqlx::query_as(
                "UPDATE matches SET \
                  event_id = COALESCE($1, event_id), \
                  mission_id = COALESCE($2, mission_id), \
                  terrain = COALESCE($3, terrain), \
                  started_at = COALESCE($4, started_at), \
                  ended_at = COALESCE($5, ended_at), \
                  outcome = $6, finalized_at = COALESCE(finalized_at, CASE WHEN $6::mission_outcome <> 'pending' THEN clock_timestamp() END), \
                  winning_faction = COALESCE($7, winning_faction), \
                  aar_replay_url = COALESCE($8, aar_replay_url) \
                 WHERE id = $9 \
                 RETURNING event_id, mission_id",
            )
            .bind(event_id)
            .bind(mission_id)
            .bind(terrain)
            .bind(m.started_at)
            .bind(m.ended_at)
            .bind(outcome)
            .bind(coalesce_str(&m.winning_faction))
            .bind(aar_replay_url)
            .bind(id)
            // `event_id` / `mission_id` are `COALESCE($n, <stored>)`, so this statement can trip
            // their constraints too once they land — a correction re-POST is exactly where a
            // wrong pointer arrives. Same mapping as the create path below.
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;
            let retract_from = match (prior_event, prior_mission) {
                (Some(old_e), Some(old_m)) if merged != (Some(old_e), Some(old_m)) => {
                    Some((old_e, old_m))
                }
                _ => None,
            };
            return Ok((id, retract_from));
        }
    }

    // `started_at` is NOT NULL, so a create with no `started_at` has to invent one. This lives
    // here rather than at the top of the function on purpose: it is a create-only fallback, and
    // the moment it is in scope beside the UPDATE above, binding it there instead of
    // `m.started_at` looks correct and silently re-times every partial retry.
    let started = m.started_at.unwrap_or_else(Utc::now);

    // On create, an absent winner/AAR still stores `''` rather than NULL — the stored match model
    // decodes both as a non-optional `String`, so a NULL would break the read path.
    let row: (Uuid, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO matches \
         (source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, \
          winning_faction, aar_replay_url, created_at, finalized_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, ''), COALESCE($9, ''), now(), CASE WHEN $7::mission_outcome <> 'pending' THEN clock_timestamp() END) \
         RETURNING id, event_id",
    )
    .bind(source_match_id)
    .bind(event_id)
    .bind(mission_id)
    .bind(terrain)
    .bind(started)
    .bind(m.ended_at)
    .bind(outcome)
    .bind(coalesce_str(&m.winning_faction))
    .bind(aar_replay_url)
    // `matches.event_id` / `matches.mission_id` have no foreign key yet. Note what the mapping
    // does and does not buy: the FK makes the write fail either way, so a 400 does not save the
    // scoreline. What it buys is that the failure is *legible and retriable* — the bridge learns
    // which pointer is wrong instead of reading "internal error", and `upsert_match` is idempotent
    // on `source_match_id`, so a corrected re-POST lands the row through the UPDATE path above.
    // Weighed against the alternative, which is a match stored with a dangling `event_id` whose
    // attendance is then silently never marked, that is the better failure — but it IS a
    // bridge-contract change and the migration must say so.
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;
    // Create has no prior pair to retract from.
    Ok((row.0, None))
}

#[cfg(test)]
#[path = "tests/match_upsert.rs"]
mod tests;
