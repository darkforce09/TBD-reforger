//! Self-service + Arma-link handlers — Rust port of `handlers/me.go`.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth;
use crate::db::refresh_leaderboard;
use crate::error::ApiError;
use crate::handlers::{is_unique_violation, load_user};
use crate::middleware::{AuthUser, ServiceAuth};
use crate::models::AuditSeverity;
use crate::services;
use crate::state::AppState;

/// 6-digit Arma link-code lifetime (10 minutes).
const LINK_CODE_TTL_MIN: i64 = 10;

/// Claim every `match_player_stats` row for an `arma_id` that no account owns yet (T-326).
///
/// `discord_id` on that table is not a fact about the row — it is a cached answer to "who owns
/// this `arma_id`", resolved once at ingest (`telemetry.rs:376-381`). Nothing ever re-asked, so
/// every match a player played *before* linking kept `discord_id = NULL` forever, and
/// `recompute_user_stats` (`telemetry.rs:530-534`) only counts non-NULL rows. Measured pre-fix
/// on the throwaway fixture: a player ingested through three real `match-results` POSTs, then
/// linked, then played a fourth — the platform's own recompute reported
/// `total_deployments = 1` for four ops, and `leaderboard_totals` showed
/// `kills = 2, deaths = 2, missions_played = 1` against true totals of 22 / 5 / 4.
///
/// `AND discord_id IS NULL` is what makes this idempotent: a second link attempt (a retried
/// confirm, a relink) finds nothing left to claim and cannot double-count. It is also why the
/// release in [`unlink`] has to exist — rows left stamped by a former owner are invisible to
/// the next owner's claim, which looks exactly like success.
///
/// `arma_id` must arrive **trimmed**. T-316 binds `p.arma_id.trim()` into this column
/// (`telemetry.rs:374`), so a padded bind matches zero rows and reports a silent success.
const BACKFILL_MATCH_STATS: &str = "UPDATE match_player_stats SET discord_id = $1 \
     WHERE arma_id = $2 AND discord_id IS NULL";

/// Mark the attendance a pre-link match could not mark (T-326 / T-431).
///
/// `event_registrations` never keyed on `arma_id` — the human registered with their Discord
/// account — so there is nothing to re-key. What is missing is the *flip*: the attendance
/// write in `ingest_match_results` (`telemetry.rs` T-230 block) only fires for players who
/// resolved at ingest, so an op played before linking leaves the registration on
/// `registered` forever. `recompute_user_stats` divides `attended` by `past_registered`,
/// so `attendance_rate` is short by exactly those ops. Fixing deployments and leaving this
/// would be half a fix.
///
/// Scoped through `s.arma_id` rather than the freshly-written `discord_id` so it states what
/// it means (the missions this Steam id actually played) and does not depend on the backfill
/// above having matched. `state <> 'attended'` only narrows the write — the end state matches
/// telemetry's unconditional `SET state = 'attended'`, including its deliberate override of a
/// `withdrawn` registration for someone who withdrew and then turned up anyway.
///
/// **T-431:** join on `(event_id, mission_id)` — the same shape as T-230 ingest
/// (`telemetry.rs` `ON m.event_id = em.event_id AND m.mission_id = em.mission_id`). Pre-fix
/// this used `em.event_id IN (SELECT m.event_id …)` and flipped *every* `event_mission` on
/// those events, so a multi-mission event marked sibling registrations `attended` when the
/// player only played one. Both columns must be non-NULL on the match: an event-only row
/// cannot know which mission was played.
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

/// `GET /api/v1/me` — the caller's user object plus their Arma-link flag.
///
/// @route GET /api/v1/me
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    let arma_linked = u.arma_id.is_some();
    Ok(Json(json!({ "user": u, "arma_linked": arma_linked })))
}

/// `PATCH /api/v1/me` — placeholder echo (profile fields come from Discord/link flow).
///
/// @route PATCH /api/v1/me
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    Ok(Json(json!({ "user": u })))
}

/// `POST /api/v1/me/link` — issue a fresh 6-digit link code (201), invalidating the
/// caller's prior unconsumed codes.
///
/// @route POST /api/v1/me/link
pub async fn create_link_code(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // Expire previous outstanding codes so only the newest is valid (best-effort).
    let _ = sqlx::query(
        "UPDATE identity_link_codes SET expires_at = now() \
         WHERE discord_id = $1 AND consumed_at IS NULL",
    )
    .bind(&user.discord_id)
    .execute(&state.pool)
    .await;

    // Generate a unique code (retry on the rare PK collision).
    for _ in 0..5 {
        let code = auth::numeric_code(6);
        let expires = Utc::now() + Duration::minutes(LINK_CODE_TTL_MIN);
        let res = sqlx::query(
            "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
             VALUES ($1, $2, $3, now())",
        )
        .bind(&code)
        .bind(&user.discord_id)
        .bind(expires)
        .execute(&state.pool)
        .await;
        match res {
            Ok(_) => {
                return Ok((
                    StatusCode::CREATED,
                    Json(json!({
                        "code": code,
                        "expires_at": crate::models::serde_helpers::go_time::format(&expires),
                    })),
                ));
            }
            Err(e) if is_unique_violation(&e) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(ApiError::internal("could not allocate code"))
}

/// `GET /api/v1/me/link/status` — link + pending-code state for UI polling.
///
/// @route GET /api/v1/me/link/status
pub async fn link_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let Some(u) = load_user(&state.pool, &user.discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_link_codes \
         WHERE discord_id = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "linked": u.arma_id.is_some(),
        "arma_id": u.arma_id,
        "arma_character": u.arma_character,
        "pending_code": pending > 0,
    })))
}

/// `DELETE /api/v1/me/link` — remove the Arma association **and release the match history
/// that link claimed** (T-326).
///
/// The release is the mirror of [`BACKFILL_MATCH_STATS`] and exists for the same reason. Once
/// the link is gone, `match_player_stats.discord_id` is a stale answer, and leaving it behind
/// breaks the *next* owner: the clash guard in [`ingest_link_confirm`] only looks at live
/// `users.arma_id`, so after an unlink a different Discord account can legitimately claim this
/// `arma_id` — and its backfill (`AND discord_id IS NULL`) then skips every row still stamped
/// by the former owner. Measured pre-fix on the throwaway fixture: after `DELETE /me/link` the
/// row stayed claimed, `total_deployments` stayed at 1 and the `leaderboard_totals` row
/// survived for an account with no Arma id at all; the second owner then linked the same
/// `arma_id` and got `total_deployments = 0` while the first kept `missions_played = 1` for a
/// match played on a Steam id they no longer own.
///
/// Release is safe *because it is reversible*: `NULL` is the known prior value, so relinking
/// re-claims exactly the same rows (proven by the retry path in the same fixture).
///
/// **Attendance is deliberately not reversed.** An `event_registrations` row was always keyed
/// on this `discord_id` — no `arma_id` was ever involved — so "they turned up to the op they
/// signed up for" stays true after they unlink a Steam account. And unlike `discord_id`, the
/// pre-flip state was never recorded (neither here nor in `telemetry.rs:418`), so a reversal
/// would have to invent one; `registered` and `withdrawn` are both plausible and picking is
/// guessing. Reversible release, non-reversible fact: only the first gets undone.
///
/// @route DELETE /api/v1/me/link
pub async fn unlink(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let mut tx = state.pool.begin().await?;
    // Read the id being severed before nulling it — the release needs it, and `FOR UPDATE`
    // stops a concurrent confirm re-linking underneath us mid-transaction.
    let arma_id: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1 FOR UPDATE")
            .bind(&user.discord_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();

    // Scoped to (this arma_id, this owner): the same row set as `discord_id` alone today, but
    // it cannot reach a row stamped from some other id if that ever becomes possible.
    let released = match &arma_id {
        Some(a) => sqlx::query(
            "UPDATE match_player_stats SET discord_id = NULL \
             WHERE arma_id = $1 AND discord_id = $2",
        )
        .bind(a)
        .bind(&user.discord_id)
        .execute(&mut *tx)
        .await?
        .rows_affected(),
        None => 0,
    };

    // arma_character is a non-null string column (app never writes NULL) → set '' not
    // NULL; wire output ("") is identical to Go, and reads still decode into String.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, arma_character = '', updated_at = now() \
         WHERE discord_id = $1",
    )
    .bind(&user.discord_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Post-commit, best-effort — the release half of the tail of [`ingest_link_confirm`], and
    // out here for the same two reasons (committed reads; a failure must not fail the request).
    if let Some(a) = &arma_id {
        if released > 0 {
            // Symmetry matters here: the claim raised `total_deployments`, so the release has to
            // lower it, or an unlinked account keeps advertising deployments whose rows no longer
            // carry its id. `attendance_rate` is recomputed too but does not move — unlink
            // deliberately leaves `event_registrations` alone (see above).
            if crate::handlers::telemetry::recompute_user_stats(&state.pool, &user.discord_id)
                .await
                .is_err()
            {
                services::write_audit(
                    &state.pool,
                    AuditSeverity::Warn,
                    None,
                    "system",
                    "user.stats_recompute_failed",
                    "User stat recompute failed after identity unlink",
                    "user",
                    &user.discord_id,
                )
                .await;
            }
            // `leaderboard_totals` aggregates `match_player_stats.discord_id` (migration
            // `0001_initial_schema.sql:270-291`), so the released rows keep counting for this
            // player on the leaderboard until the view is refreshed.
            if refresh_leaderboard(&state.pool).await.is_err() {
                services::write_audit(
                    &state.pool,
                    AuditSeverity::Warn,
                    None,
                    "system",
                    "leaderboard.refresh_failed",
                    "Leaderboard refresh failed after identity unlink",
                    "user",
                    &user.discord_id,
                )
                .await;
            }
        }
        // Releasing a service record must not be silent — without this, a player's deployment
        // count dropping to zero has no explanation anywhere in the audit log.
        let username = crate::handlers::username(&state.pool, &user.discord_id).await;
        services::write_audit(
            &state.pool,
            AuditSeverity::Info,
            Some(&user.discord_id),
            &username,
            "identity.unlink",
            &format!(
                "{username} unlinked Arma Steam ID {a}; released {released} historical match row(s)"
            ),
            "user",
            &user.discord_id,
        )
        .await;
    }

    Ok(Json(json!({ "linked": false })))
}

/// Body posted by the in-game mod (service-token authenticated).
///
/// **`arma_character` is deliberately required — do not add `#[serde(default)]` to it
/// (T-319).** Its two siblings default *and are guarded* by the emptiness check in
/// [`ingest_link_confirm`]; this one defaulted with nothing behind it, and the handler binds
/// it into `UPDATE users SET … arma_character = $2` unconditionally. Same shape as T-185 and
/// T-218: the default is not "no data", it is an affirmative empty string that overwrites.
///
/// Measured pre-fix on the dev fixture: a confirm carrying `code` + `arma_id` but no
/// `arma_character` returned 200 `{"linked":true}` and took `[TBD] Dev Operator` to `''`.
/// The account stays linked, so nothing ever re-runs this write — the name is simply gone
/// until someone unlinks and relinks. That silence is the whole problem, and a 400 is the
/// cheap end of it: the confirm fails loudly, the code is still live, the mod retries.
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
/// the match history that id already accumulated (T-326).
///
/// @route POST /api/v1/ingest/link-confirm
pub async fn ingest_link_confirm(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<LinkConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    // Names `arma_character` too: after T-319 it must be *present*, so an omitted one arrives
    // here as a decode error and a 400 listing only the other two misdirects the caller.
    let Json(req) =
        body.map_err(|_| ApiError::bad_request("code, arma_id and arma_character are required"))?;

    // Trim once, here, and bind *only* this form below (T-326). The column this links against
    // is written trimmed by T-316 (`telemetry.rs:374`), and every path that reads it binds
    // trimmed too (`telemetry.rs:376-381`). Storing the raw form broke all three of them at
    // once, and measurably: a confirm carrying `"  76561198000000999  "` stored
    // `users.arma_id = "  76561198000000999  "` while the player's rows were stored as
    // `"76561198000000999"`, so the ingest resolver found **no user** for that Steam id — the
    // account read as linked in the UI and was invisible to every future match ingest, forever.
    // The backfill would have inherited exactly that: zero rows matched, 200 OK, nothing wrong
    // to see. `trim().is_empty()` replaces `is_empty()` for the same reason it did in T-218 and
    // T-316 — `"   "` is not an Arma id, and it passed the old check straight into the column.
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
    // the code is single-use and, once consumed, gone — so if the backfill were run after the
    // commit and failed, the player would be linked with no history and no way to retry, which
    // is strictly worse than the bug being fixed. Inside the transaction a failure rolls the
    // consume back with it and the mod's next retry of the *same* code works. Verified on the
    // throwaway fixture with a `BEFORE UPDATE` trigger on `match_player_stats` that raises: the
    // confirm returned 500, the code stayed unconsumed, `users.arma_id` stayed NULL, and
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

    // The missing step. Nothing else in the crate ever writes `match_player_stats.discord_id`
    // after ingest, so without these two statements the rows stay orphaned for good.
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
    // `ingest_match_results` — that one propagates a recompute error with `?`
    // (`telemetry.rs:432`), which is fine for an idempotent endpoint whose sender just re-POSTs.
    // Here the link code is already spent, so a 500 after the commit would send the mod back to a
    // 404 on retry and show the player a failed link that actually succeeded. The rows are correct
    // either way; these two only refresh derived numbers, and the next match ingest redoes both.
    if claimed > 0 || attended > 0 {
        // `users.total_deployments` / `attendance_rate` are denormalized, and this is the crate's
        // only definition of them (see the visibility note on `recompute_user_stats`). Without
        // this call a player who links after their *last* op reads zero deployments forever,
        // because nothing else would ever recount.
        if crate::handlers::telemetry::recompute_user_stats(&state.pool, &discord_id)
            .await
            .is_err()
        {
            services::write_audit(
                &state.pool,
                AuditSeverity::Warn,
                None,
                "system",
                "user.stats_recompute_failed",
                "User stat recompute failed after identity link backfill",
                "user",
                &discord_id,
            )
            .await;
        }
        // `leaderboard_totals` reads `match_player_stats.discord_id` directly
        // (`0001_initial_schema.sql:270-291`), so a refresh is all the leaderboard needs; it has
        // no `arma_id` of its own to backfill.
        if refresh_leaderboard(&state.pool).await.is_err() {
            services::write_audit(
                &state.pool,
                AuditSeverity::Warn,
                None,
                "system",
                "leaderboard.refresh_failed",
                "Leaderboard refresh failed after identity link backfill",
                "user",
                &discord_id,
            )
            .await;
        }
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
    services::write_audit(
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
mod tests {
    use super::*;

    /// T-431 Class-R — identity-link attendance backfill must scope through
    /// `(event_id, mission_id)`, matching T-230 ingest. Playing one mission on a
    /// multi-mission event must NOT flip sibling `event_mission` registrations.
    ///
    /// RED: restore the pre-T-431 nest
    ///   `SELECT em.id FROM event_missions em WHERE em.event_id IN (SELECT m.event_id …)`
    ///   (drop `m.mission_id = em.mission_id`) → this test FAIL.
    /// GREEN: the JOIN pin below is present and the event-only nest is absent.
    #[test]
    fn backfill_attendance_joins_event_id_and_mission_id() {
        // Assembled so a bait comment / this test's source cannot false-green the const.
        let join_pin = format!(
            "{}{}",
            "m.event_id = em.event_id AND ", "m.mission_id = em.mission_id"
        );
        assert!(
            BACKFILL_ATTENDANCE.contains(&join_pin),
            "BACKFILL_ATTENDANCE must join event_missions on (event_id, mission_id) \
             like T-230 ingest (perturbation: drop mission_id from the JOIN)"
        );
        assert!(
            BACKFILL_ATTENDANCE.contains("m.mission_id IS NOT NULL"),
            "BACKFILL_ATTENDANCE must refuse event-only matches (no mission to scope)"
        );

        // Forbidden pre-T-431 shape: every event_mission on the event, not the played one.
        let event_only_nest = format!("{}{}", "em.event_id IN (", " SELECT m.event_id");
        let collapsed: String = BACKFILL_ATTENDANCE
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !collapsed.contains(
                &event_only_nest
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            "BACKFILL_ATTENDANCE must not nest `em.event_id IN (SELECT m.event_id …)` — \
             that flips sibling missions on multi-mission events"
        );

        // Source pin (production only) — same join must appear on the const, not only in docs.
        const SRC: &str = include_str!("me.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("me.rs must have a #[cfg(test)] module");
        assert!(
            production.contains(&join_pin),
            "production BACKFILL_ATTENDANCE source must contain `{join_pin}`"
        );
        assert!(
            !production.contains("WHERE em.event_id IN ("),
            "production must not keep the pre-T-431 event_id-only nest"
        );
    }
}
