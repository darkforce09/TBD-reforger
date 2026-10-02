//! Operator side of the ledger: accept a command (its intent is durable before any executor
//! can act on it), cancel one that no executor has claimed, and read receipts.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::command_arguments::validated_arguments;
use crate::administration::services::required_audit::append_actor_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::fleet_command::{
    FleetAction, FleetCommandReceipt, FleetCommandRequest, FleetCommandState,
};

/// Seconds an accepted command waits for an executor before it expires.
pub const QUEUED_LIFETIME_SECONDS: i64 = 300;

pub(super) const RECEIPT_COLUMNS: &str = "id, server_id, executor_kind, action, arguments, \
     requested_by, requested_at, expires_at, state, attempts, claimed_at, executing_at, \
     finished_at, outcome, failure_reason";

pub(super) fn state_conflict(code: &str, message: String, state: &str) -> ApiError {
    ApiError::with_details(
        axum::http::StatusCode::CONFLICT,
        message,
        serde_json::json!({ "code": code, "state": state }),
    )
}

/// Accept a command for a server the caller has locked, recording the operator's intent.
pub async fn enqueue_command(
    connection: &mut PgConnection,
    server_id: Uuid,
    request: &FleetCommandRequest,
    actor: &str,
) -> Result<FleetCommandReceipt, ApiError> {
    let (arguments, bound_session) = validated_arguments(request.action, &request.arguments)?;
    if let Some(session) = bound_session {
        let open: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM server_runtime_sessions
                 WHERE id = $1 AND server_id = $2 AND ended_at IS NULL)",
        )
        .bind(session)
        .bind(server_id)
        .fetch_one(&mut *connection)
        .await?;
        if !open {
            return Err(ApiError::with_details(
                axum::http::StatusCode::CONFLICT,
                "the runtime session is not this server's open session",
                serde_json::json!({ "code": "RUNTIME_SESSION_ENDED" }),
            ));
        }
    }
    insert_command(connection, server_id, request.action, &arguments, actor).await
}

/// Accept a command a mission deployment issues with arguments it built and validated; the
/// deployment and the command commit together.
pub async fn enqueue_deployment_command(
    connection: &mut PgConnection,
    server_id: Uuid,
    action: FleetAction,
    arguments: &serde_json::Value,
    actor: &str,
) -> Result<FleetCommandReceipt, ApiError> {
    if !action.deployment_only() {
        return Err(ApiError::internal(
            "only deployment actions are issued by deployments",
        ));
    }
    insert_command(connection, server_id, action, arguments, actor).await
}

/// Record the command and its audit entry.
async fn insert_command(
    connection: &mut PgConnection,
    server_id: Uuid,
    action: FleetAction,
    arguments: &serde_json::Value,
    actor: &str,
) -> Result<FleetCommandReceipt, ApiError> {
    let receipt: FleetCommandReceipt = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO fleet_commands (server_id, executor_kind, action, arguments, idempotent,
             process_changing, requested_by, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, clock_timestamp() + make_interval(secs => $8))
         RETURNING {RECEIPT_COLUMNS}"
    )))
    .bind(server_id)
    .bind(action.executor().as_str())
    .bind(action.as_str())
    .bind(sqlx::types::Json(arguments))
    .bind(action.idempotent())
    .bind(action.process_changing())
    .bind(actor)
    .bind(QUEUED_LIFETIME_SECONDS as f64)
    .fetch_one(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        actor,
        "server.command_requested",
        "fleet_command",
        &receipt.id.to_string(),
        &format!(
            "Requested {} on server {server_id} with {arguments}",
            action.as_str()
        ),
    )
    .await?;
    Ok(receipt)
}

/// Cancel a command no executor has claimed. Any other state answers 409 with that state.
pub async fn cancel_command(
    connection: &mut PgConnection,
    server_id: Uuid,
    command: Uuid,
    actor: &str,
) -> Result<FleetCommandReceipt, ApiError> {
    let state: String = sqlx::query_scalar(
        "SELECT state FROM fleet_commands WHERE id = $1 AND server_id = $2 FOR NO KEY UPDATE",
    )
    .bind(command)
    .bind(server_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("command not found"))?;
    if state == FleetCommandState::Cancelled.as_str() {
        return load_receipt_on(connection, server_id, command).await;
    }
    if state != FleetCommandState::Queued.as_str() {
        return Err(state_conflict(
            "COMMAND_NOT_CANCELLABLE",
            format!("a {state} command can no longer be cancelled"),
            &state,
        ));
    }
    let receipt: FleetCommandReceipt = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE fleet_commands SET state = 'cancelled', finished_at = clock_timestamp(),
             failure_reason = 'cancelled by an administrator'
         WHERE id = $1 RETURNING {RECEIPT_COLUMNS}"
    )))
    .bind(command)
    .fetch_one(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        actor,
        "server.command_cancelled",
        "fleet_command",
        &command.to_string(),
        &format!("Cancelled {} on server {server_id}", receipt.action),
    )
    .await?;
    Ok(receipt)
}

async fn load_receipt_on(
    connection: &mut PgConnection,
    server_id: Uuid,
    command: Uuid,
) -> Result<FleetCommandReceipt, ApiError> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {RECEIPT_COLUMNS} FROM fleet_commands WHERE id = $1 AND server_id = $2"
    )))
    .bind(command)
    .bind(server_id)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| ApiError::not_found("command not found"))
}

pub async fn load_receipt(
    pool: &PgPool,
    server_id: Uuid,
    command: Uuid,
) -> Result<FleetCommandReceipt, ApiError> {
    let mut connection = pool.acquire().await?;
    load_receipt_on(&mut connection, server_id, command).await
}

/// The server's commands, newest first.
pub async fn list_receipts(
    pool: &PgPool,
    server_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<FleetCommandReceipt>, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {RECEIPT_COLUMNS} FROM fleet_commands WHERE server_id = $1
         ORDER BY requested_at DESC, id DESC LIMIT $2 OFFSET $3"
    )))
    .bind(server_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?)
}
