//! The player-facing half of the Arma link handshake: issuing a 6-digit code, reporting
//! link state, and severing the link again.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{Duration, Utc};
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::authentication_primitives;
use crate::core::database::postgres_errors::is_unique_violation;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::identity_and_access::services::session_issuance::arma_id_is_linked;
use crate::identity_and_access::services::user_lookup::load_user;
use crate::services;

/// 6-digit Arma link-code lifetime (10 minutes).
const LINK_CODE_TTL_MIN: i64 = 10;

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
        let code = authentication_primitives::numeric_code(6);
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
                        "expires_at": crate::core::wire_format::go_time::format(&expires),
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
        "linked": arma_id_is_linked(&u.arma_id),
        "arma_id": u.arma_id,
        "arma_character": u.arma_character,
        "pending_code": pending > 0,
    })))
}

/// `DELETE /api/v1/me/link` — remove the Arma association **and release the match history
/// that link claimed**.
///
/// The release is the mirror of the `BACKFILL_MATCH_STATS` claim in
/// [`super::arma_link_confirmation`] and exists for the same reason. Once the link is gone,
/// `match_player_stats.discord_id` is a stale answer, and leaving it behind breaks the *next*
/// owner: the clash guard in `ingest_link_confirm` only looks at live `users.arma_id`, so
/// after an unlink a different Discord account can legitimately claim this `arma_id` — and
/// its backfill (`AND discord_id IS NULL`) then skips every row still stamped by the former
/// owner. Without the release the row stays claimed, `total_deployments` keeps counting for an
/// account with no Arma id at all, and the second owner links the same `arma_id` to zero
/// history.
///
/// Release is safe *because it is reversible*: `NULL` is the known prior value, so relinking
/// re-claims exactly the same rows.
///
/// **Attendance is deliberately not reversed.** An `event_registrations` row was always keyed
/// on this `discord_id` — no `arma_id` was ever involved — so "they turned up to the op they
/// signed up for" stays true after they unlink a Steam account. And unlike `discord_id`, the
/// pre-flip state is never recorded anywhere, so a reversal would have to invent one;
/// `registered` and `withdrawn` are both plausible and picking is guessing. Reversible
/// release, non-reversible fact: only the first gets undone.
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

    // arma_character is a non-null string column (the app never writes NULL) → set '' not
    // NULL; the wire output ("") is unchanged and reads still decode into String.
    sqlx::query(
        "UPDATE users SET arma_id = NULL, arma_character = '', updated_at = now() \
         WHERE discord_id = $1",
    )
    .bind(&user.discord_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Post-commit, best-effort — the release half of the tail of `ingest_link_confirm`, and
    // out here for the same two reasons (committed reads; a failure must not fail the request).
    if let Some(a) = &arma_id {
        if released > 0 {
            // Symmetry matters here: the claim raised `total_deployments`, so the release has to
            // lower it, or an unlinked account keeps advertising deployments whose rows no longer
            // carry its id. `attendance_rate` is recomputed too but does not move — unlink
            // deliberately leaves `event_registrations` alone (see above).
            services::recompute_user_stats_best_effort(
                &state.pool,
                &user.discord_id,
                "User stat recompute failed after identity unlink",
            )
            .await;
            // `leaderboard_totals` aggregates `match_player_stats.discord_id` (migration
            // `0001_initial_schema.sql:270-291`), so the released rows keep counting for this
            // player on the leaderboard until the view is refreshed.
            services::refresh_leaderboard_best_effort(
                &state.pool,
                "Leaderboard refresh failed after identity unlink",
                "user",
                &user.discord_id,
            )
            .await;
        }
        // Releasing a service record must not be silent — without this, a player's deployment
        // count dropping to zero has no explanation anywhere in the audit log.
        let username = actor_display_name(&state.pool, &user.discord_id).await;
        write_audit(
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
