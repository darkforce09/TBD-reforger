//! Access credentials identify a session; PostgreSQL decides its current permissions.
//!
//! **Role:** turns verified access token claims into the [`AuthUser`] a request acts as, from
//! the session row and the account's current authority.
//! **Position:** [`DatabaseSessionAuthority`] is the session authority the application state
//! holds for the [`AuthUser`] extractor; [`authorize_on_connection`] rechecks a caller inside the
//! business transactions of missions, operations and server infrastructure.
//! **Signals & state:** none; each check reads the database on the pool or connection it is
//! given.
//! **Invariants:** an expired, revoked or deleted session never authorizes; a development role
//! authorizes only a development-configured API; a banned account has no role.

use crate::account_authority::{ACCOUNT_AUTHORITY_QUERY, AccountAuthority};
use api_configuration::configuration::Config;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::authentication_primitives::Claims;
use chrono::Utc;
use sqlx::{PgConnection, PgPool};

use crate::arma_identity_link::arma_id_is_linked;
use api_http_layer::middleware::AuthUser;

/// [`authorize_on_connection`] on a connection of its own.
pub async fn authorize_session(
    pool: &PgPool,
    config: &Config,
    claims: &Claims,
) -> Result<AuthUser, ApiError> {
    let mut connection = pool.acquire().await?;
    authorize_on_connection(&mut connection, config, claims).await
}

/// Recheck a business transaction after it locks the account row.
pub async fn authorize_on_connection(
    connection: &mut PgConnection,
    config: &Config,
    claims: &Claims,
) -> Result<AuthUser, ApiError> {
    if claims.exp <= Utc::now().timestamp() {
        return Err(ApiError::unauthorized("expired access token"));
    }
    // Composition contains only static SQL; all request values remain bind parameters.
    let query = format!(
        "SELECT a.*, s.expires_at AS session_expires_at,
        s.revoked_at AS session_revoked_at, s.development_role
        FROM ({ACCOUNT_AUTHORITY_QUERY}) a JOIN authentication_sessions s USING (discord_id)
        WHERE s.id = $3"
    );
    let session: AuthorizedSession = sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
        .bind(&claims.sub)
        .bind(&config.discord_guild_id)
        .bind(claims.sid)
        .fetch_optional(connection)
        .await?
        .ok_or_else(|| ApiError::unauthorized("invalid session"))?;
    if claims.exp <= Utc::now().timestamp()
        || session.session_revoked_at.is_some()
        || session.session_expires_at <= session.account.observed_at
        || session.account.deleted_at.is_some()
        || (session.development_role.is_some() && !config.is_development())
    {
        return Err(ApiError::unauthorized("expired or revoked session"));
    }
    let account = &session.account;
    let decision = account.permissions(account.observed_at);
    let role = decision
        .effective_role
        .ok_or_else(|| ApiError::forbidden("account is banned"))?;
    Ok(AuthUser {
        discord_id: account.discord_id.clone(),
        role: session.development_role.unwrap_or(role).as_str().to_owned(),
        arma_linked: arma_id_is_linked(&account.arma_id),
        session_claims: claims.clone(),
        membership_stale: session.development_role.is_none() && decision.stale,
        membership_override_active: decision.override_active,
        can_manage_membership_override: account.can_manage_override(account.observed_at),
    })
}

#[derive(sqlx::FromRow)]
struct AuthorizedSession {
    #[sqlx(flatten)]
    account: AccountAuthority,
    session_expires_at: chrono::DateTime<Utc>,
    session_revoked_at: Option<chrono::DateTime<Utc>>,
    development_role: Option<crate::UserRole>,
}

/// The session authority backed by PostgreSQL, held by the application state.
pub struct DatabaseSessionAuthority {
    /// The pool each check acquires its connection from.
    pub pool: PgPool,
    /// The configuration the application state shares: the guild and the development flag.
    pub config: std::sync::Arc<Config>,
}

impl api_http_layer::authentication_primitives::session_authority::SessionAuthority
    for DatabaseSessionAuthority
{
    fn authorize(
        &self,
        claims: Claims,
    ) -> futures::future::BoxFuture<'static, Result<AuthUser, ApiError>> {
        let pool = self.pool.clone();
        let config = self.config.clone();
        Box::pin(async move { authorize_session(&pool, &config, &claims).await })
    }
}
