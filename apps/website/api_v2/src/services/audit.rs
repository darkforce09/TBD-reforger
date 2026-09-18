//! Audit log writer — Rust port of `services/audit.go`.

use sqlx::PgPool;

use crate::models::AuditSeverity;

/// Append a row to the audit log. Best-effort: an audit failure must not break the
/// primary action, so this returns `()` and logs on error (callers ignore it, as in
/// Go). `id` is a bigint sequence; `created_at` is set app-side.
#[allow(clippy::too_many_arguments)]
pub async fn write_audit(
    pool: &PgPool,
    severity: AuditSeverity,
    actor_id: Option<&str>,
    actor_name: &str,
    action: &str,
    message: &str,
    target_type: &str,
    target_id: &str,
) {
    let res = sqlx::query(
        "INSERT INTO audit_logs \
         (severity, actor_id, actor_name, action, message, target_type, target_id, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
    )
    .bind(severity)
    .bind(actor_id)
    .bind(actor_name)
    .bind(action)
    .bind(message)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::error!(action, error = %e, "audit write failed");
    }
}
