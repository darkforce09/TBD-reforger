//! Required audit records share the business transaction and its publication enqueue trigger.
//!
//! **Role:** appends the audit line a business write must not happen without.
//! **Position:** called by the handlers and services of every domain, and by the staging fixtures
//! host tool, on the connection of the transaction that makes the change.
//! **Signals & state:** none; each append runs on the caller's connection.
//! **Invariants:** the line commits or rolls back with its business change; an actor append
//! fails with [`sqlx::Error::RowNotFound`] when the actor's account does not exist, so a line is
//! never attributed to nobody.

use crate::AuditSeverity;
use api_identifiers::{AuditTargetId, DiscordUserId};
use sqlx::PgConnection;

/// [`append_actor_audit`] on a `user` target: the account an identity or session change touches.
pub async fn append_required_audit(
    connection: &mut PgConnection,
    actor_id: &DiscordUserId,
    action: &str,
    target_id: impl Into<AuditTargetId>,
    message: &str,
) -> sqlx::Result<()> {
    append_actor_audit(connection, actor_id, action, "user", target_id, message).await
}

/// Authenticated mutations retain the actor and the actual business target in their transaction.
pub async fn append_actor_audit(
    connection: &mut PgConnection,
    actor_id: &DiscordUserId,
    action: &str,
    target_type: &str,
    target_id: impl Into<AuditTargetId>,
    message: &str,
) -> sqlx::Result<()> {
    append_actor_audit_with_severity(
        connection,
        AuditSeverity::Info,
        actor_id,
        action,
        target_type,
        target_id,
        message,
    )
    .await
}

/// [`append_actor_audit`] for actions that warrant another severity, such as bans.
pub async fn append_actor_audit_with_severity(
    connection: &mut PgConnection,
    severity: AuditSeverity,
    actor_id: &DiscordUserId,
    action: &str,
    target_type: &str,
    target_id: impl Into<AuditTargetId>,
    message: &str,
) -> sqlx::Result<()> {
    let inserted = sqlx::query("INSERT INTO audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, created_at)
        SELECT $1, discord_id, CASE WHEN btrim(username) = '' THEN discord_id ELSE username END,
            $2, $3, $6, $4, now() FROM users WHERE discord_id = $5")
        .bind(severity).bind(action).bind(message).bind(Into::<AuditTargetId>::into(target_id)).bind(actor_id).bind(target_type)
        .execute(connection).await?;
    if inserted.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

/// Machine-originated business events use the same transaction without inventing an account actor.
///
/// `audit_logs.created_at` has no column default, so the row stamps `now()`, the time of the
/// transaction that appends it, exactly as the actor appends do.
pub async fn append_system_audit(
    connection: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: impl Into<AuditTargetId>,
    message: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs(severity, actor_name, action, message, target_type, target_id, created_at)
        VALUES ($1, 'system', $2, $3, $4, $5, now())",
    )
    .bind(AuditSeverity::Info)
    .bind(action)
    .bind(message)
    .bind(target_type)
    .bind(Into::<AuditTargetId>::into(target_id))
    .execute(connection)
    .await?;
    Ok(())
}
