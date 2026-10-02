//! Session persistence. Writers hold the account lock before session and token locks.

use crate::core::authentication_primitives::{hash_token, random_token};
use crate::identity_and_access::models::user_account::UserRole;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

pub const SESSION_LIFETIME: Duration = Duration::days(30);

#[derive(sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub discord_id: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub development_role: Option<UserRole>,
}

#[derive(sqlx::FromRow)]
pub struct RefreshRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn create_session(
    connection: &mut PgConnection,
    discord_id: &str,
    development_role: Option<UserRole>,
) -> sqlx::Result<Uuid> {
    let id = Uuid::new_v4();
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

pub async fn insert_refresh(
    connection: &mut PgConnection,
    discord_id: &str,
    session_id: Uuid,
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

pub async fn revoke_session(connection: &mut PgConnection, session_id: Uuid) -> sqlx::Result<()> {
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
    discord_id: &str,
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
