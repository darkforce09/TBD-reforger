//! Dev-login handler — Rust port of `handlers/dev.go`. Development-only shortcut
//! that mints a session without Discord, redirecting to the SPA callback with the
//! token fragment exactly like the real callback.

use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;

use crate::error::ApiError;
use crate::handlers::auth::{issue_session, session_redirect};
use crate::state::AppState;

/// Stable Discord snowflake for the local dev operator.
const DEV_USER_ID: &str = "000000000000000001";

#[derive(Debug, Deserialize)]
pub struct DevLoginQuery {
    #[serde(default)]
    role: String,
}

/// `GET /api/v1/auth/dev-login?role=admin|mission_maker|leader|enlisted`.
///
/// @route GET /api/v1/auth/dev-login
pub async fn dev_login(
    State(state): State<AppState>,
    Query(q): Query<DevLoginQuery>,
) -> Result<Response, ApiError> {
    // Registered only in development, but re-guard at request time like Go.
    if !state.cfg.is_development() {
        return Err(ApiError::not_found("not found"));
    }

    let role = match q.role.as_str() {
        r @ ("enlisted" | "leader" | "mission_maker" | "admin") => r,
        _ => "admin",
    };

    // Upsert the dev user. On conflict, only username/handle/role/last_login/updated
    // change (matching Go's DoUpdates); avatar/arma stay as first inserted.
    sqlx::query(
        "INSERT INTO users \
         (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, \
          is_banned, ban_reason, last_login_at, created_at, updated_at) \
         VALUES ($1, 'Dev Operator', 'devoperator', '', 'dev-arma-76561190000000001', \
          '[TBD] Dev Operator', $2::user_role, false, '', now(), now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle, \
          role = EXCLUDED.role, last_login_at = EXCLUDED.last_login_at, updated_at = now()",
    )
    .bind(DEV_USER_ID)
    .bind(role)
    .execute(&state.pool)
    .await?;

    let (access, exp, refresh) = issue_session(&state, DEV_USER_ID, role, true).await?;
    Ok(session_redirect(
        &state.cfg.frontend_url,
        &access,
        &refresh,
        exp,
        true,
    ))
}
