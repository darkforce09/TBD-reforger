//! Session persistence. Writers hold the account lock before session and token locks.

use api_caller_identity::UserRole;
use api_http_layer::authentication_primitives::{hash_token, random_token};
use api_identifiers::{AuthenticationSessionId, DiscordUserId, RefreshTokenId};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

/// How long a session and its refresh tokens stay valid after issuance.
pub const SESSION_LIFETIME: Duration = Duration::days(30);

/// An `authentication_sessions` row.
#[derive(sqlx::FromRow)]
pub struct SessionRow {
    /// The session id.
    pub id: AuthenticationSessionId,
    /// The Discord user id of the session's account.
    pub discord_id: DiscordUserId,
    /// When the session expires.
    pub expires_at: DateTime<Utc>,
    /// When the session was revoked, if it was.
    pub revoked_at: Option<DateTime<Utc>>,
    /// The role a development login session acts as; absent for a Discord session.
    pub development_role: Option<UserRole>,
}

/// A `refresh_tokens` row.
#[derive(sqlx::FromRow)]
pub struct RefreshRow {
    /// The refresh token's row id.
    pub id: RefreshTokenId,
    /// The session the token renews.
    pub session_id: AuthenticationSessionId,
    /// When the token expires.
    pub expires_at: DateTime<Utc>,
    /// When the token was revoked, if it was.
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Creates a session for `discord_id`, valid for [`SESSION_LIFETIME`], and answers its id.
pub async fn create_session(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
    development_role: Option<UserRole>,
) -> sqlx::Result<AuthenticationSessionId> {
    let id = AuthenticationSessionId::new(Uuid::new_v4());
    sqlx::query(
        "INSERT INTO authentication_sessions (id, discord_id, expires_at, development_role)
        VALUES ($1, $2, clock_timestamp() + $3 * interval '1 second', $4)",
    )
    .bind(id)
    .bind(discord_id)
    .bind(SESSION_LIFETIME.num_seconds() as f64)
    .bind(development_role)
    .execute(connection)
    .await?;
    Ok(id)
}

/// Mints a refresh token for `session_id`, stores its hash, and answers the raw token.
pub async fn insert_refresh(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
    session_id: AuthenticationSessionId,
) -> sqlx::Result<String> {
    let raw = random_token(32);
    sqlx::query(
        "INSERT INTO refresh_tokens (discord_id, session_id, token_hash, expires_at, created_at)
        VALUES ($1, $2, $3, clock_timestamp() + $4 * interval '1 second', clock_timestamp())",
    )
    .bind(discord_id)
    .bind(session_id)
    .bind(hash_token(&raw))
    .bind(SESSION_LIFETIME.num_seconds() as f64)
    .execute(connection)
    .await?;
    Ok(raw)
}

/// Revokes the session and every refresh token it holds; an earlier revocation time is kept.
pub async fn revoke_session(
    connection: &mut PgConnection,
    session_id: AuthenticationSessionId,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE authentication_sessions SET revoked_at = COALESCE(revoked_at, clock_timestamp()) WHERE id = $1",
    )
    .bind(session_id)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = COALESCE(revoked_at, clock_timestamp()) WHERE session_id = $1",
    )
    .bind(session_id)
    .execute(connection)
    .await?;
    Ok(())
}

/// Replay revokes all sessions belonging to the account, including concurrently issued successors.
pub async fn revoke_account_sessions(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE authentication_sessions SET revoked_at = COALESCE(revoked_at, clock_timestamp()) WHERE discord_id = $1")
        .bind(discord_id).execute(&mut *connection).await?;
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = COALESCE(revoked_at, clock_timestamp()) WHERE discord_id = $1",
    )
    .bind(discord_id)
    .execute(connection)
    .await?;
    Ok(())
}
