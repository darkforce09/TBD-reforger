//! Settling a server's deployment in flight. A runtime session of the server started after the
//! request decides it by the artifact it reports loading: the deployment's artifact with its
//! exact document SHA-256 confirms it, anything else fails it. Without such a report, a fleet
//! command that ended without effect fails it, and so does a passed deadline, as a partial
//! transition. Callers settle under the server row lock, which session starts also take.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;

#[derive(sqlx::FromRow)]
struct InFlight {
    id: Uuid,
    artifact_id: Uuid,
    document_sha256: String,
    requested_at: chrono::DateTime<chrono::Utc>,
    overdue: bool,
    command_action: String,
    command_state: String,
    command_failure: Option<String>,
}

/// The first artifact report of a session started after the request.
#[derive(sqlx::FromRow)]
struct RuntimeReport {
    session: Uuid,
    loaded_artifact_id: Uuid,
    loaded_artifact_sha256: String,
}

async fn finish(
    connection: &mut PgConnection,
    deployment: Uuid,
    confirmed_by: Option<Uuid>,
    failure: Option<&str>,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE mission_deployments SET state = CASE WHEN $2::uuid IS NULL THEN 'failed' ELSE 'confirmed' END,
             confirmed_runtime_session_id = $2, failure_reason = $3, finished_at = clock_timestamp()
         WHERE id = $1",
    )
    .bind(deployment)
    .bind(confirmed_by)
    .bind(failure)
    .execute(&mut *connection)
    .await?;
    let (action, message) = match (confirmed_by, failure) {
        (Some(session), _) => (
            "mission.deployment_confirmed",
            format!("Runtime session {session} confirmed the deployed artifact"),
        ),
        (None, reason) => (
            "mission.deployment_failed",
            format!("Deployment failed: {}", reason.unwrap_or_default()),
        ),
    };
    append_system_audit(
        connection,
        action,
        "mission_deployment",
        &deployment.to_string(),
        &message,
    )
    .await?;
    Ok(())
}

/// Settle the server's deployment in flight, if any. The caller holds the server row lock.
pub async fn settle_server_deployment(
    connection: &mut PgConnection,
    server: Uuid,
) -> Result<(), ApiError> {
    let in_flight: Option<InFlight> = sqlx::query_as(
        "SELECT d.id, d.artifact_id, a.document_sha256, d.requested_at,
             d.deadline_at <= clock_timestamp() AS overdue, c.action AS command_action,
             c.state AS command_state, c.failure_reason AS command_failure
         FROM mission_deployments d JOIN mission_artifacts a ON a.id = d.artifact_id
         JOIN fleet_commands c ON c.id = d.fleet_command_id
         WHERE d.server_id = $1 AND d.state = 'requested' FOR NO KEY UPDATE OF d",
    )
    .bind(server)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(deployment) = in_flight else {
        return Ok(());
    };
    let report: Option<RuntimeReport> = sqlx::query_as(
        "SELECT id AS session, loaded_artifact_id, loaded_artifact_sha256
         FROM server_runtime_sessions
         WHERE server_id = $1 AND started_at >= $2 AND loaded_artifact_id IS NOT NULL
         ORDER BY started_at, id LIMIT 1",
    )
    .bind(server)
    .bind(deployment.requested_at)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(report) = report {
        if report.loaded_artifact_id == deployment.artifact_id
            && report.loaded_artifact_sha256 == deployment.document_sha256
        {
            return finish(connection, deployment.id, Some(report.session), None).await;
        }
        let reason = format!(
            "runtime session {} loaded artifact {} ({}) instead of {} ({})",
            report.session,
            report.loaded_artifact_id,
            report.loaded_artifact_sha256,
            deployment.artifact_id,
            deployment.document_sha256
        );
        return finish(connection, deployment.id, None, Some(&reason)).await;
    }
    if matches!(
        deployment.command_state.as_str(),
        "failed" | "expired" | "cancelled" | "indeterminate"
    ) {
        let reason = format!(
            "the {} command ended {}: {}",
            deployment.command_action,
            deployment.command_state,
            deployment
                .command_failure
                .as_deref()
                .unwrap_or("no reason recorded")
        );
        return finish(connection, deployment.id, None, Some(&reason)).await;
    }
    if deployment.overdue {
        let reason = format!(
            "partial transition: no runtime session reported the artifact before the deadline; the {} command is {}",
            deployment.command_action, deployment.command_state
        );
        return finish(connection, deployment.id, None, Some(&reason)).await;
    }
    Ok(())
}

/// Lock the server and settle its deployment in flight.
pub async fn lock_and_settle(connection: &mut PgConnection, server: Uuid) -> Result<(), ApiError> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM servers WHERE id = $1 FOR NO KEY UPDATE")
        .bind(server)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or_else(|| ApiError::not_found("server not found"))?;
    settle_server_deployment(connection, server).await
}

/// Settle every server's deployment in flight, one transaction per server. Returns how many
/// servers had one.
pub async fn reconcile_mission_deployments(pool: &PgPool) -> Result<usize, ApiError> {
    let servers: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT server_id FROM mission_deployments WHERE state = 'requested'",
    )
    .fetch_all(pool)
    .await?;
    for server in &servers {
        let mut transaction = pool.begin().await?;
        lock_and_settle(&mut transaction, *server).await?;
        transaction.commit().await?;
    }
    Ok(servers.len())
}
