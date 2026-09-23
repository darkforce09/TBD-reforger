//! Crash reconciliation by time: a queued command nobody claimed expires; a claim whose lease
//! lapsed before its effect started returns to the queue under a new fencing token; a lease that
//! lapsed during the effect returns an idempotent command to the queue and makes any other
//! command indeterminate, which nothing repeats.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReconciliationOutcome {
    pub expired: usize,
    pub requeued: usize,
    pub indeterminate: usize,
}

#[derive(sqlx::FromRow)]
struct Lapsed {
    id: Uuid,
    action: String,
    state: String,
    idempotent: bool,
    expired: bool,
}

/// One pass over every server. Rows another transaction holds are skipped and examined on the
/// next pass.
pub async fn reconcile_fleet_commands(pool: &PgPool) -> Result<ReconciliationOutcome, ApiError> {
    let mut transaction = pool.begin().await?;
    let mut outcome = ReconciliationOutcome::default();
    let lapsed: Vec<Lapsed> = sqlx::query_as(
        "SELECT id, action, state, idempotent, expires_at <= clock_timestamp() AS expired
         FROM fleet_commands
         WHERE (state = 'queued' AND expires_at <= clock_timestamp())
            OR (state IN ('claimed', 'executing') AND lease_expires_at <= clock_timestamp())
         ORDER BY id FOR NO KEY UPDATE SKIP LOCKED",
    )
    .fetch_all(&mut *transaction)
    .await?;
    for command in lapsed {
        let (state, reason) = match (command.state.as_str(), command.idempotent, command.expired) {
            ("executing", false, _) => (
                "indeterminate",
                "the executor stopped reporting after the effect started; the outcome is unknown",
            ),
            (_, _, true) => (
                "expired",
                "no executor completed the command before it expired",
            ),
            _ => (
                "queued",
                "the executor's lease lapsed; the command returns to the queue",
            ),
        };
        if state == "queued" {
            sqlx::query(
                "UPDATE fleet_commands SET state = 'queued', claimed_by = NULL, claimed_at = NULL,
                     lease_expires_at = NULL, executing_at = NULL
                 WHERE id = $1",
            )
            .bind(command.id)
            .execute(&mut *transaction)
            .await?;
            outcome.requeued += 1;
        } else {
            sqlx::query(
                "UPDATE fleet_commands SET state = $2, finished_at = clock_timestamp(),
                     failure_reason = $3, claimed_by = NULL, lease_expires_at = NULL
                 WHERE id = $1",
            )
            .bind(command.id)
            .bind(state)
            .bind(reason)
            .execute(&mut *transaction)
            .await?;
            if state == "expired" {
                outcome.expired += 1;
            } else {
                outcome.indeterminate += 1;
            }
        }
        append_system_audit(
            &mut transaction,
            &format!("server.command_{state}"),
            "fleet_command",
            &command.id.to_string(),
            &format!("{} was {}: {reason}", command.action, command.state),
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(outcome)
}
