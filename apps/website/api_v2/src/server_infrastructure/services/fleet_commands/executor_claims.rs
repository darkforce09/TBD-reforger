//! Executor side of the ledger: claim the next command under a lease and a fencing token, report
//! that its effect is starting, and report its outcome. Every report must carry the current
//! fencing token of the claiming credential, so an executor whose lease lapsed cannot overwrite
//! the command after another claim. Lock order: server, runtime session, command rows.

use sqlx::PgConnection;
use uuid::Uuid;

use super::command_ledger::{RECEIPT_COLUMNS, state_conflict};
use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::identity_and_access::services::account_authority::holds_administrator_authority;
use crate::server_infrastructure::models::fleet_command::{
    ClaimedFleetCommand, ExecutionResult, FleetAction, FleetCommandReceipt, FleetCommandState,
};
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;
use crate::server_infrastructure::services::runtime_sessions::share_open_session;

/// Seconds a claim stays valid before its executor reports that the effect is starting.
pub const CLAIM_LEASE_SECONDS: i64 = 30;

#[derive(sqlx::FromRow)]
struct Candidate {
    id: Uuid,
    action: String,
    arguments: sqlx::types::Json<serde_json::Value>,
    requested_by: String,
    process_changing: bool,
}

async fn finish(
    connection: &mut PgConnection,
    command: Uuid,
    state: FleetCommandState,
    reason: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE fleet_commands SET state = $2, finished_at = clock_timestamp(), failure_reason = $3,
             claimed_by = NULL, lease_expires_at = NULL
         WHERE id = $1",
    )
    .bind(command)
    .bind(state.as_str())
    .bind(reason)
    .execute(&mut *connection)
    .await?;
    append_system_audit(
        connection,
        &format!("server.command_{}", state.as_str()),
        "fleet_command",
        &command.to_string(),
        reason,
    )
    .await?;
    Ok(())
}

/// Claim the caller's next command, oldest first. A game runtime names its open session;
/// commands bound to another session of the server fail instead of reaching a new session.
pub async fn claim_next_command(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    runtime_session: Option<Uuid>,
    main_guild: &str,
) -> Result<Option<ClaimedFleetCommand>, ApiError> {
    // Claims of one server serialize, so the one-process-change rule reads committed state.
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM servers WHERE id = $1 AND is_active FOR NO KEY UPDATE",
    )
    .bind(caller.server_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::forbidden("the credential's server is deactivated"))?;
    if caller.executor == ExecutorKind::ModRuntime {
        let session = runtime_session.ok_or_else(|| {
            ApiError::bad_request("a game runtime claims within its runtime session")
        })?;
        share_open_session(connection, caller, session).await?;
        let stale: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fleet_commands
             WHERE server_id = $1 AND executor_kind = 'mod_runtime' AND state = 'queued'
               AND arguments ? 'runtime_session_id'
               AND arguments ->> 'runtime_session_id' <> $2::text
             ORDER BY id FOR NO KEY UPDATE SKIP LOCKED",
        )
        .bind(caller.server_id)
        .bind(session.to_string())
        .fetch_all(&mut *connection)
        .await?;
        for command in stale {
            finish(
                connection,
                command,
                FleetCommandState::Failed,
                "the runtime session the command was issued against has ended",
            )
            .await?;
        }
    }
    loop {
        let candidate: Option<Candidate> = sqlx::query_as(
            "SELECT id, action, arguments, requested_by, process_changing FROM fleet_commands
             WHERE server_id = $1 AND executor_kind = $2 AND state = 'queued'
               AND expires_at > clock_timestamp()
               AND (NOT process_changing OR NOT EXISTS (
                   SELECT 1 FROM fleet_commands running WHERE running.server_id = $1
                     AND running.process_changing AND running.state IN ('claimed', 'executing')))
             ORDER BY requested_at, id LIMIT 1 FOR NO KEY UPDATE SKIP LOCKED",
        )
        .bind(caller.server_id)
        .bind(caller.executor.as_str())
        .fetch_optional(&mut *connection)
        .await?;
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        if !holds_administrator_authority(connection, &candidate.requested_by, main_guild).await? {
            finish(
                connection,
                candidate.id,
                FleetCommandState::Cancelled,
                "the requester no longer holds administrator authority",
            )
            .await?;
            continue;
        }
        let (fencing_token, lease_expires_at): (i64, chrono::DateTime<chrono::Utc>) =
            sqlx::query_as(
                "UPDATE fleet_commands SET state = 'claimed', fencing_token = fencing_token + 1,
                     claimed_by = $2, claimed_at = clock_timestamp(), attempts = attempts + 1,
                     lease_expires_at = clock_timestamp() + make_interval(secs => $3)
                 WHERE id = $1 RETURNING fencing_token, lease_expires_at",
            )
            .bind(candidate.id)
            .bind(caller.credential_id)
            .bind(CLAIM_LEASE_SECONDS as f64)
            .fetch_one(&mut *connection)
            .await?;
        append_system_audit(
            connection,
            "server.command_claimed",
            "fleet_command",
            &candidate.id.to_string(),
            &format!(
                "Credential {} claimed {} with fencing token {fencing_token}{}",
                caller.credential_id,
                candidate.action,
                if candidate.process_changing {
                    " (process change)"
                } else {
                    ""
                }
            ),
        )
        .await?;
        return Ok(Some(ClaimedFleetCommand {
            command_id: candidate.id,
            server_id: caller.server_id,
            action: candidate.action,
            arguments: candidate.arguments.0,
            fencing_token,
            lease_expires_at,
        }));
    }
}

#[derive(sqlx::FromRow)]
struct Claim {
    server_id: Uuid,
    action: String,
    state: String,
    fencing_token: i64,
    claimed_by: Option<Uuid>,
}

/// Lock the command and require the caller's current claim of it.
async fn lock_claim(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    command: Uuid,
    fencing_token: i64,
) -> Result<Claim, ApiError> {
    let claim: Claim = sqlx::query_as(
        "SELECT server_id, action, state, fencing_token, claimed_by FROM fleet_commands
         WHERE id = $1 FOR NO KEY UPDATE",
    )
    .bind(command)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("command not found"))?;
    caller.require_server(claim.server_id)?;
    if claim.fencing_token != fencing_token || claim.claimed_by != Some(caller.credential_id) {
        return Err(ApiError::with_details(
            axum::http::StatusCode::CONFLICT,
            "the claim is no longer current",
            serde_json::json!({ "code": "STALE_FENCING_TOKEN", "state": claim.state }),
        ));
    }
    Ok(claim)
}

async fn receipt(
    connection: &mut PgConnection,
    command: Uuid,
) -> Result<FleetCommandReceipt, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {RECEIPT_COLUMNS} FROM fleet_commands WHERE id = $1"
    )))
    .bind(command)
    .fetch_one(connection)
    .await?)
}

/// Record that the effect is starting. The intent is durable before the executor acts, so a
/// lapse from here on can make a non-idempotent command indeterminate but never repeat it.
pub async fn mark_executing(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    command: Uuid,
    fencing_token: i64,
) -> Result<FleetCommandReceipt, ApiError> {
    let claim = lock_claim(connection, caller, command, fencing_token).await?;
    if claim.state == FleetCommandState::Executing.as_str() {
        return receipt(connection, command).await;
    }
    if claim.state != FleetCommandState::Claimed.as_str() {
        return Err(state_conflict(
            "COMMAND_NOT_CLAIMED",
            format!("the command is {}", claim.state),
            &claim.state,
        ));
    }
    let window = FleetAction::parse(&claim.action)
        .ok_or_else(|| ApiError::internal("unknown fleet action"))?
        .execution_window_seconds();
    sqlx::query(
        "UPDATE fleet_commands SET state = 'executing', executing_at = clock_timestamp(),
             lease_expires_at = clock_timestamp() + make_interval(secs => $2)
         WHERE id = $1",
    )
    .bind(command)
    .bind(window as f64)
    .execute(&mut *connection)
    .await?;
    append_system_audit(
        connection,
        "server.command_executing",
        "fleet_command",
        &command.to_string(),
        &format!(
            "Credential {} started {}",
            caller.credential_id, claim.action
        ),
    )
    .await?;
    receipt(connection, command).await
}

/// Record the outcome the executor observed. Success is accepted only for a command whose
/// effect was reported as starting; a failure may also end a claim before that.
pub async fn record_result(
    connection: &mut PgConnection,
    caller: &MachineCaller,
    command: Uuid,
    result: &ExecutionResult,
) -> Result<FleetCommandReceipt, ApiError> {
    let claim = lock_claim(connection, caller, command, result.fencing_token).await?;
    let allowed = if result.succeeded {
        claim.state == FleetCommandState::Executing.as_str()
    } else {
        claim.state == FleetCommandState::Executing.as_str()
            || claim.state == FleetCommandState::Claimed.as_str()
    };
    if !allowed {
        return Err(state_conflict(
            "COMMAND_NOT_EXECUTING",
            format!("the command is {}", claim.state),
            &claim.state,
        ));
    }
    let reason = match (&result.failure_reason, result.succeeded) {
        (_, true) => None,
        (Some(reason), false) if !reason.trim().is_empty() && reason.len() <= 512 => {
            Some(reason.trim().to_owned())
        }
        (_, false) => {
            return Err(ApiError::bad_request(
                "a failed outcome names a failure_reason of 1 to 512 bytes",
            ));
        }
    };
    let state = if result.succeeded {
        FleetCommandState::Succeeded
    } else {
        FleetCommandState::Failed
    };
    let outcome = result.outcome.clone().map(serde_json::Value::Object);
    sqlx::query(
        "UPDATE fleet_commands SET state = $2, finished_at = clock_timestamp(), outcome = $3,
             failure_reason = $4, claimed_by = NULL, lease_expires_at = NULL
         WHERE id = $1",
    )
    .bind(command)
    .bind(state.as_str())
    .bind(outcome.map(sqlx::types::Json))
    .bind(&reason)
    .execute(&mut *connection)
    .await?;
    append_system_audit(
        connection,
        &format!("server.command_{}", state.as_str()),
        "fleet_command",
        &command.to_string(),
        &format!(
            "Credential {} reported {} {}{}",
            caller.credential_id,
            claim.action,
            state.as_str(),
            reason
                .map(|reason| format!(": {reason}"))
                .unwrap_or_default()
        ),
    )
    .await?;
    receipt(connection, command).await
}
