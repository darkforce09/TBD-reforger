//! The mod-facing half of the Arma link handshake: `POST /ingest/link-confirm` spends a
//! pending code, attaches the Arma id, and claims the match history that id already
//! accumulated.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::command_center::services::user_stats;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::ServiceAuth;
use crate::identity_and_access::services::user_lookup::load_user;

/// Claim every `match_player_stats` row for an `arma_id` that no account owns yet.
///
/// `discord_id` on that table is not a fact about the row — it is a cached answer to "who owns
/// this `arma_id`", resolved once at ingest. Nothing ever re-asks, so without this every match
/// a player played *before* linking would keep `discord_id = NULL` forever, and
/// `recompute_user_stats` (`command_center/services/user_stats.rs`) only counts non-NULL rows. Measured on a
/// throwaway fixture without the claim: a player ingested through three real `match-results`
/// POSTs, then linked, then played a fourth — the platform's own recompute reported
/// `total_deployments = 1` for four ops, and `leaderboard_totals` showed
/// `kills = 2, deaths = 2, missions_played = 1` against true totals of 22 / 5 / 4.
///
/// `AND discord_id IS NULL` is what makes this idempotent: a second link attempt (a retried
/// confirm, a relink) finds nothing left to claim and cannot double-count. It is also why the
/// release in `super::arma_link_codes::unlink` has to exist — rows left stamped by a former
/// owner are invisible to the next owner's claim, which looks exactly like success.
///
/// `arma_id` must arrive **trimmed**. Ingest binds the trimmed id into this column, so a
/// padded bind matches zero rows and reports a silent success.
const BACKFILL_MATCH_STATS: &str = "UPDATE match_player_stats SET discord_id = $1 \
     WHERE arma_id = $2 AND discord_id IS NULL";

/// Mark the attendance a pre-link match could not mark.
///
/// `event_registrations` never keyed on `arma_id` — the human registered with their Discord
/// account — so there is nothing to re-key. What is missing is the *flip*: the attendance
/// write in `ingest_match_results` only fires for players who resolved at ingest, so an op
/// played before linking leaves the registration on `registered` forever.
/// `recompute_user_stats` divides `attended` by `past_registered`, so `attendance_rate` is
/// short by exactly those ops. Fixing deployments and leaving this would be half a fix.
///
/// Scoped through `s.arma_id` rather than the freshly-written `discord_id` so it states what
/// it means (the missions this Steam id actually played) and does not depend on the backfill
/// above having matched. `state <> 'attended'` only narrows the write — the end state matches
/// telemetry's unconditional `SET state = 'attended'`, including its deliberate override of a
/// `withdrawn` registration for someone who withdrew and then turned up anyway.
///
/// The join is on `(event_id, mission_id)` — the same shape as ingest. An event-id-only nest
/// flips *every* `event_mission` on those events, so a multi-mission event marks sibling
/// registrations `attended` when the player only played one. Both columns must be non-NULL on
/// the match: an event-only row cannot know which mission was played.
const BACKFILL_ATTENDANCE: &str = "UPDATE event_registrations SET state = 'attended' \
     WHERE discord_id = $1 AND state <> 'attended' \
       AND event_mission_id IN ( \
         SELECT em.id FROM event_missions em \
         INNER JOIN matches m \
           ON m.event_id = em.event_id AND m.mission_id = em.mission_id \
         INNER JOIN match_player_stats s ON s.match_id = m.id \
         WHERE s.arma_id = $2 \
           AND m.event_id IS NOT NULL \
           AND m.mission_id IS NOT NULL)";

/// Body posted by the in-game mod (service-token authenticated).
///
/// **`arma_character` is deliberately required — do not add `#[serde(default)]` to it.** Its
/// two siblings default *and are guarded* by the emptiness check in [`ingest_link_confirm`];
/// this one has nothing behind it, and the handler binds it into
/// `UPDATE users SET … arma_character = $2` unconditionally. The default is not "no data", it
/// is an affirmative empty string that overwrites.
///
/// Measured on the dev fixture with the default in place: a confirm carrying `code` +
/// `arma_id` but no `arma_character` returned 200 `{"linked":true}` and took
/// `[TBD] Dev Operator` to `''`. The account stays linked, so nothing ever re-runs this write
/// — the name is simply gone until someone unlinks and relinks. That silence is the whole
/// problem, and a 400 is the cheap end of it: the confirm fails loudly, the code is still
/// live, the mod retries.
///
/// The live caller already sends the field unconditionally
/// (`apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_IdentityLink.c` `BuildPayload`
/// appends all three), so requiring it costs the mod nothing and only closes the door on a
/// payload that has *lost* the field. An explicit `""` still writes `""` — that is a stated
/// intent and matches `unlink`, which parks the column at `''` on purpose.
#[derive(Debug, Deserialize)]
pub struct LinkConfirmRequest {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub arma_id: String,
    pub arma_character: String,
}

/// `POST /api/v1/ingest/link-confirm` — consume a pending code, attach the Arma id, and claim
/// the match history that id already accumulated.
///
/// @route POST /api/v1/ingest/link-confirm
pub async fn ingest_link_confirm(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<LinkConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    // Names `arma_character` too: it must be *present*, so an omitted one arrives here as a
    // decode error and a 400 listing only the other two misdirects the caller.
    let Json(req) =
        body.map_err(|_| ApiError::bad_request("code, arma_id and arma_character are required"))?;

    // Trim once, here, and bind *only* this form below. The column this links against is
    // written trimmed by ingest, and every path that reads it binds trimmed too. Storing the
    // raw form breaks all three at once, and measurably: a confirm carrying
    // `"  76561198000000999  "` stored `users.arma_id = "  76561198000000999  "` while the
    // player's rows were stored as `"76561198000000999"`, so the ingest resolver found **no
    // user** for that Steam id — the account read as linked in the UI and was invisible to
    // every future match ingest, forever. The backfill inherits exactly that: zero rows
    // matched, 200 OK, nothing wrong to see. `trim().is_empty()` rather than `is_empty()` for
    // the same reason — `"   "` is not an Arma id, and it passes a bare emptiness check
    // straight into the column.
    let arma_id = req.arma_id.trim();
    if req.code.is_empty() || arma_id.is_empty() {
        return Err(ApiError::bad_request("code and arma_id required"));
    }

    // Look up a live (unconsumed, unexpired) code.
    let found: Option<(String, String)> = sqlx::query_as(
        "SELECT code, discord_id FROM identity_link_codes \
         WHERE code = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(&req.code)
    .fetch_optional(&state.pool)
    .await?;
    let Some((code, discord_id)) = found else {
        return Err(ApiError::not_found("invalid or expired code"));
    };

    // Guard against linking an Arma ID already owned by another account.
    let clash: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE arma_id = $1 AND discord_id <> $2")
            .bind(arma_id)
            .bind(&discord_id)
            .fetch_one(&state.pool)
            .await?;
    if clash > 0 {
        return Err(ApiError::conflict(
            "arma id already linked to another account",
        ));
    }

    // One transaction spends the code and claims the history together. The ordering matters:
    // the code is single-use and, once consumed, gone — so if the backfill ran after the
    // commit and failed, the player would be linked with no history and no way to retry, which
    // is strictly worse than the problem being solved. Inside the transaction a failure rolls
    // the consume back with it and the mod's next retry of the *same* code works. Verified on
    // a throwaway fixture with a `BEFORE UPDATE` trigger on `match_player_stats` that raises:
    // the confirm returned 500, the code stayed unconsumed, `users.arma_id` stayed NULL, and
    // re-POSTing that same code after dropping the trigger linked and claimed all three rows.
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE identity_link_codes SET consumed_at = now(), arma_id = $1 WHERE code = $2")
        .bind(arma_id)
        .bind(&code)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE users SET arma_id = $1, arma_character = $2, updated_at = now() \
         WHERE discord_id = $3",
    )
    .bind(arma_id)
    .bind(&req.arma_character)
    .bind(&discord_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal("could not link identity"))?;

    // Nothing else in the crate ever writes `match_player_stats.discord_id` after ingest, so
    // without these two statements the rows stay orphaned for good.
    let claimed = sqlx::query(BACKFILL_MATCH_STATS)
        .bind(&discord_id)
        .bind(arma_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("could not link identity"))?
        .rows_affected();
    let attended = sqlx::query(BACKFILL_ATTENDANCE)
        .bind(&discord_id)
        .bind(arma_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("could not link identity"))?
        .rows_affected();

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("could not link identity"))?;

    // Post-commit, best-effort — mirrors the tail of `ingest_match_results`.
    //
    // Both of these belong out here, for the same reason: they read committed state.
    // `recompute_user_stats` takes a `&PgPool`, so inside the transaction it would count on a
    // different connection and miss the rows just claimed; `REFRESH MATERIALIZED VIEW
    // CONCURRENTLY` cannot run in a transaction at all.
    //
    // Neither may fail the request, which is where this deliberately diverges from
    // `ingest_match_results` — that one propagates a recompute error with `?`, which is fine
    // for an idempotent endpoint whose sender just re-POSTs. Here the link code is already
    // spent, so a 500 after the commit would send the mod back to a 404 on retry and show the
    // player a failed link that actually succeeded. The rows are correct either way; these two
    // only refresh derived numbers, and the next match ingest redoes both.
    if claimed > 0 || attended > 0 {
        // `users.total_deployments` / `attendance_rate` are denormalized, and
        // `user_stats::recompute_user_stats` is the crate's only definition of them. Without this
        // a player who links after their *last* op reads zero deployments forever, because
        // nothing else would ever recount.
        user_stats::recompute_user_stats_best_effort(
            &state.pool,
            &discord_id,
            "User stat recompute failed after identity link backfill",
        )
        .await;
        // `leaderboard_totals` reads `match_player_stats.discord_id` directly
        // (`0001_initial_schema.sql:270-291`), so a refresh is all the leaderboard needs; it has
        // no `arma_id` of its own to backfill.
        user_stats::refresh_leaderboard_best_effort(
            &state.pool,
            "Leaderboard refresh failed after identity link backfill",
            "user",
            &discord_id,
        )
        .await;
    }

    // Best-effort audit (username reload; failure must not fail the request). The counts are
    // named because the backfill rewrites history: a deployment total that jumps on link needs
    // a trace saying why.
    let username = load_user(&state.pool, &discord_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(&discord_id),
        &username,
        "identity.link",
        &format!(
            "{username} successfully linked their Arma Steam ID; \
             backfilled {claimed} historical match row(s) and {attended} attendance record(s)"
        ),
        "user",
        &discord_id,
    )
    .await;

    Ok(Json(json!({
        "linked": true,
        "discord_id": discord_id,
        "arma_id": arma_id,
        "arma_character": req.arma_character,
    })))
}

#[cfg(test)]
#[path = "tests/arma_link_confirmation.rs"]
mod tests;
