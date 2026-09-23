//! Required audit records share the business transaction and its publication enqueue trigger.

use crate::administration::models::audit_log::AuditSeverity;
use sqlx::PgConnection;

pub async fn append_required_audit(
    connection: &mut PgConnection,
    actor_id: &str,
    action: &str,
    target_id: &str,
    message: &str,
) -> sqlx::Result<()> {
    append_actor_audit(connection, actor_id, action, "user", target_id, message).await
}

/// Authenticated mutations retain the actor and the actual business target in their transaction.
pub async fn append_actor_audit(
    connection: &mut PgConnection,
    actor_id: &str,
    action: &str,
    target_type: &str,
    target_id: &str,
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
    actor_id: &str,
    action: &str,
    target_type: &str,
    target_id: &str,
    message: &str,
) -> sqlx::Result<()> {
    let inserted = sqlx::query("INSERT INTO audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, created_at)
        SELECT $1, discord_id, CASE WHEN btrim(username) = '' THEN discord_id ELSE username END,
            $2, $3, $6, $4, now() FROM users WHERE discord_id = $5")
        .bind(severity).bind(action).bind(message).bind(target_id).bind(actor_id).bind(target_type)
        .execute(connection).await?;
    if inserted.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

/// Machine-originated business events use the same transaction without inventing an account actor.
pub async fn append_system_audit(
    connection: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: &str,
    message: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs(severity, actor_name, action, message, target_type, target_id)
        VALUES ($1, 'system', $2, $3, $4, $5)",
    )
    .bind(AuditSeverity::Info)
    .bind(action)
    .bind(message)
    .bind(target_type)
    .bind(target_id)
    .execute(connection)
    .await?;
    Ok(())
}
