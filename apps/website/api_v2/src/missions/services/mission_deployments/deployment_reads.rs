//! Reading deployments: a server's deployments as operators observe them, and the deployment a
//! server's runtime runs.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::missions::models::mission_deployment::{
    MissionDeployment, MissionDeploymentPage, RuntimeDeployment,
};

const DEPLOYMENT_VIEW: &str = "SELECT d.id, d.server_id, d.mission_id, m.title AS mission_title, \
     d.artifact_id, a.artifact_digest, a.document_sha256 AS artifact_sha256, d.event_mission_id, \
     d.terrain_key, d.scenario_id, d.transition, d.fleet_command_id, \
     c.state AS fleet_command_state, d.requested_by, d.requested_via, d.requested_at, \
     d.deadline_at, d.state, d.confirmed_runtime_session_id, d.finished_at, d.failure_reason, \
     (SELECT count(*) FROM mission_deployment_slots b WHERE b.deployment_id = d.id) AS bound_slots \
     FROM mission_deployments d JOIN missions m ON m.id = d.mission_id \
     JOIN mission_artifacts a ON a.id = d.artifact_id \
     JOIN fleet_commands c ON c.id = d.fleet_command_id";

pub async fn load_deployment(
    connection: &mut PgConnection,
    server: Uuid,
    deployment: Uuid,
) -> Result<MissionDeployment, ApiError> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{DEPLOYMENT_VIEW} WHERE d.id = $1 AND d.server_id = $2"
    )))
    .bind(deployment)
    .bind(server)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| ApiError::not_found("deployment not found"))
}

/// The server's deployments, newest first.
pub async fn list_deployments(
    connection: &mut PgConnection,
    server: Uuid,
    limit: i64,
    offset: i64,
) -> Result<MissionDeploymentPage, ApiError> {
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mission_deployments WHERE server_id = $1")
            .bind(server)
            .fetch_one(&mut *connection)
            .await?;
    let items: Vec<MissionDeployment> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{DEPLOYMENT_VIEW} WHERE d.server_id = $1 ORDER BY d.requested_at DESC, d.id DESC
         LIMIT $2 OFFSET $3"
    )))
    .bind(server)
    .bind(limit)
    .bind(offset)
    .fetch_all(connection)
    .await?;
    Ok(MissionDeploymentPage {
        items,
        total,
        limit,
        offset,
    })
}

/// The deployment the server runs: the one in flight, else the latest confirmed one.
pub async fn deployment_in_effect(
    connection: &mut PgConnection,
    server: Uuid,
) -> Result<Option<RuntimeDeployment>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT d.id AS deployment_id, d.state, d.mission_id, d.artifact_id,
             a.document_sha256 AS artifact_sha256, a.document_bytes AS artifact_bytes,
             d.terrain_key, d.scenario_id, em.event_id, d.event_mission_id
         FROM mission_deployments d JOIN mission_artifacts a ON a.id = d.artifact_id
         LEFT JOIN event_missions em ON em.id = d.event_mission_id
         WHERE d.server_id = $1 AND d.state IN ('requested', 'confirmed')
         ORDER BY d.state = 'requested' DESC, d.finished_at DESC NULLS LAST, d.requested_at DESC
         LIMIT 1",
    )
    .bind(server)
    .fetch_optional(connection)
    .await?)
}
