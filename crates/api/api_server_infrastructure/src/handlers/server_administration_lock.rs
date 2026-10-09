//! The lock-then-reauthorize step every administrator write to one server's fleet state starts
//! with.
//!
//! **Role:** locks the `servers` row and confirms, after the lock wait, that the caller is still
//! an administrator.
//! **Position:** server infrastructure handlers; the machine credential routes
//! ([`super::machine_credentials`]) and the fleet command routes ([`super::fleet_commands`]) call
//! [`lock_server_as_admin`] first in their transaction.
//! **Signals & state:** none; the row lock lives in the caller's transaction.
//! **Invariants:** the role is read on the locking connection after the lock is held, so a
//! demotion committed during the wait refuses the write (403); an unknown server answers 404
//! before any role check.

use api_caller_identity::session_authorization::authorize_on_connection;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::{AdminUser, role_rank};
use api_identifiers::{DiscordUserId, ServerId};
use api_state::AppState;
use sqlx::PgConnection;

/// Lock the server, then confirm the caller is still an administrator after the lock wait.
/// Returns the administrator and whether the server is active.
pub(super) async fn lock_server_as_admin(
    connection: &mut PgConnection,
    state: &AppState,
    admin: &AdminUser,
    server: ServerId,
) -> Result<(DiscordUserId, bool), ApiError> {
    let active: bool =
        sqlx::query_scalar("SELECT is_active FROM servers WHERE id = $1 FOR NO KEY UPDATE")
            .bind(server)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or_else(|| ApiError::not_found("server not found"))?;
    let actor = authorize_on_connection(connection, &state.cfg, &admin.0.session_claims).await?;
    if role_rank(&actor.role) < role_rank("admin") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    Ok((actor.discord_id, active))
}
