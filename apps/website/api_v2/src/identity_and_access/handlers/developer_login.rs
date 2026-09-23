//! Development-only login shortcut: mints a session without Discord, redirecting to the SPA
//! callback with the token fragment exactly like the real callback.

use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::identity_and_access::services::session_issuance::{
    issue_development_session, session_redirect,
};

/// Stable Discord snowflake for the local **admin** / default-role operator.
///
/// Each role gets its own discord_id (see [`discord_id_for_role`]). Sharing one id across
/// roles makes `ON CONFLICT` rewrite `role` on the same row and lets `issue_session` stack
/// refresh families without revoking the prior one.
const DEV_USER_ID: &str = "000000000000000001";

const DEV_USER_ID_GUEST: &str = "000000000000000005";
const DEV_USER_ID_ENLISTED: &str = "000000000000000002";
const DEV_USER_ID_LEADER: &str = "000000000000000003";
const DEV_USER_ID_MISSION_MAKER: &str = "000000000000000004";

/// Stable Arma id for the local **admin** operator. Applied only on first create (see
/// [`dev_login`]) so concurrent cold inserts cannot race `idx_users_arma_id`.
///
/// Role-specific arma ids (see [`arma_id_for_role`]) keep distinct rows from racing the
/// same unique index when several roles cold-insert in parallel.
const DEV_ARMA_ID: &str = "dev-arma-76561190000000001";

const DEV_ARMA_ID_GUEST: &str = "dev-arma-76561190000000005";
const DEV_ARMA_ID_ENLISTED: &str = "dev-arma-76561190000000002";
const DEV_ARMA_ID_LEADER: &str = "dev-arma-76561190000000003";
const DEV_ARMA_ID_MISSION_MAKER: &str = "dev-arma-76561190000000004";

#[derive(Debug, Deserialize)]
pub struct DevLoginQuery {
    #[serde(default)]
    role: String,
}

fn discord_id_for_role(role: &str) -> &'static str {
    match role {
        "guest" => DEV_USER_ID_GUEST,
        "enlisted" => DEV_USER_ID_ENLISTED,
        "leader" => DEV_USER_ID_LEADER,
        "mission_maker" => DEV_USER_ID_MISSION_MAKER,
        // "admin" and unknown→admin (normalized before call)
        _ => DEV_USER_ID,
    }
}

fn arma_id_for_role(role: &str) -> &'static str {
    match role {
        "guest" => DEV_ARMA_ID_GUEST,
        "enlisted" => DEV_ARMA_ID_ENLISTED,
        "leader" => DEV_ARMA_ID_LEADER,
        "mission_maker" => DEV_ARMA_ID_MISSION_MAKER,
        _ => DEV_ARMA_ID,
    }
}

/// `GET /api/v1/auth/dev-login?role=admin|mission_maker|leader|enlisted`.
///
/// @route GET /api/v1/auth/dev-login
pub async fn dev_login(
    State(state): State<AppState>,
    Query(q): Query<DevLoginQuery>,
) -> Result<Response, ApiError> {
    // Registered only in development, but re-guard at request time.
    if !state.cfg.is_development() {
        return Err(ApiError::not_found("not found"));
    }

    let role = match q.role.as_str() {
        r @ ("guest" | "enlisted" | "leader" | "mission_maker" | "admin") => r,
        _ => "admin",
    };
    let discord_id = discord_id_for_role(role);
    let arma_id = arma_id_for_role(role);

    // Upsert the role's dedicated row. On conflict, only username/handle/role/last_login/updated
    // change; avatar/arma stay as first inserted.
    //
    // Do NOT stamp a FIXED `arma_id` into this INSERT. `ON CONFLICT (discord_id)` arbitrates
    // only that index; concurrent first-time inserts against a cold DB both take the INSERT
    // path and the loser trips `idx_users_arma_id` (23505 → 500). NULL is allowed many times
    // on that unique index, so the upsert itself cannot collide. Per-role discord_id means
    // concurrent *different* roles do not rewrite one shared row; per-role arma_id keeps their
    // COALESCE first-creates from racing each other.
    sqlx::query(
        "INSERT INTO users \
         (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, \
          is_banned, ban_reason, last_login_at, created_at, updated_at) \
         VALUES ($1, 'Dev Operator', 'devoperator', '', NULL, \
          '[TBD] Dev Operator', $2::user_role, false, '', now(), now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle, \
          role = EXCLUDED.role, last_login_at = EXCLUDED.last_login_at, updated_at = now()",
    )
    .bind(discord_id)
    .bind(role)
    .execute(&state.pool)
    .await?;

    // First creator fills arma_id; concurrent losers keep whatever is already there.
    sqlx::query(
        "UPDATE users SET arma_id = COALESCE(arma_id, $2), updated_at = now() \
         WHERE discord_id = $1",
    )
    .bind(discord_id)
    .bind(arma_id)
    .execute(&state.pool)
    .await?;

    let role = serde_json::from_value(serde_json::json!(role))
        .map_err(|_| ApiError::bad_request("invalid development role"))?;
    let (access, exp, refresh) = issue_development_session(&state, discord_id, role).await?;
    Ok(session_redirect(
        &state.cfg.frontend_url,
        &access,
        &refresh,
        exp,
        true,
    ))
}
