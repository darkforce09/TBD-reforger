//! Game-runtime mission routes (`mod_runtime` machine credential, the credential's own server):
//! what the runtime runs, the exact bytes of an artifact deployed to it, the missions an in-game
//! administrator may deploy to it, and an in-game administrator's deployment request.

use api_identifiers::MissionArtifactId;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{Json, Response};
use fleet_wire_contract::ExecutorKind;
use serde_json::json;

use crate::handlers::artifact_document_response::artifact_document_response;
use crate::models::mission_deployment::{
    DeployableMission, DeployableMissionList, MissionDeployment, RelayedDeploymentRequest,
    RuntimeDeployment,
};
use crate::services::mission_artifacts::artifact_store::load_artifact_document;
use crate::services::mission_deployments::deployment_reads::deployment_in_effect;
use crate::services::mission_deployments::deployment_requests::{Requester, request_deployment};
use crate::services::mission_deployments::deployment_settlement::lock_and_settle;
use api_caller_identity::machine_caller::MachineCaller;
use api_failpoints::fail_point;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_state::AppState;

/// `GET /api/v1/game-runtime/deployment` — the deployment this server runs: the one in flight,
/// else the latest confirmed one. 404 `NO_DEPLOYMENT` when the server has none.
///
/// @route GET /api/v1/game-runtime/deployment
pub async fn current_deployment(
    State(state): State<AppState>,
    caller: MachineCaller,
) -> Result<Json<RuntimeDeployment>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let mut transaction = state.pool.begin().await?;
    lock_and_settle(&mut transaction, caller.server_id).await?;
    let deployment = deployment_in_effect(&mut transaction, caller.server_id).await?;
    transaction.commit().await?;
    deployment.map(Json).ok_or_else(|| {
        ApiError::with_details(
            StatusCode::NOT_FOUND,
            "no mission is deployed to this server",
            json!({ "code": "NO_DEPLOYMENT" }),
        )
    })
}

/// `GET /api/v1/game-runtime/artifacts/:artifactId` — the exact compiled bytes of an artifact a
/// deployment of this server names, with their SHA-256 as a strong entity tag.
///
/// @route GET /api/v1/game-runtime/artifacts/:artifactId
pub async fn deployed_artifact_document(
    State(state): State<AppState>,
    caller: MachineCaller,
    PathParams(artifact): PathParams<String>,
) -> Result<Response, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let artifact = artifact
        .parse::<MissionArtifactId>()
        .map_err(|_| ApiError::bad_request("invalid artifact id"))?;
    let mut connection = state.pool.acquire().await?;
    let deployed: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM mission_deployments
             WHERE server_id = $1 AND artifact_id = $2 AND state IN ('requested', 'confirmed'))",
    )
    .bind(caller.server_id)
    .bind(artifact)
    .fetch_one(&mut *connection)
    .await?;
    if !deployed {
        return Err(ApiError::with_details(
            StatusCode::FORBIDDEN,
            "no deployment of this server names the artifact",
            json!({ "code": "ARTIFACT_NOT_DEPLOYED_HERE" }),
        ));
    }
    let document = load_artifact_document(&mut connection, artifact).await?;
    Ok(artifact_document_response(document))
}

/// `GET /api/v1/game-runtime/missions` — every live mission whose approved artifact this server
/// can run: compiled against the server's required modpack, on a terrain with a registered
/// scenario.
///
/// @route GET /api/v1/game-runtime/missions
pub async fn deployable_missions(
    State(state): State<AppState>,
    caller: MachineCaller,
) -> Result<Json<DeployableMissionList>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let missions: Vec<DeployableMission> = sqlx::query_as(
        "SELECT m.id AS mission_id, m.title, a.terrain AS terrain_key, a.id AS artifact_id,
             a.document_sha256 AS artifact_sha256
         FROM missions m JOIN mission_artifacts a ON a.id = m.approved_artifact_id
         JOIN servers s ON s.id = $1
         JOIN fleet_scenarios f ON f.terrain_key = a.terrain
         WHERE m.status = 'live' AND m.deleted_at IS NULL
           AND a.modpack_id IS NOT DISTINCT FROM s.required_modpack_id
         ORDER BY m.title, m.id",
    )
    .bind(caller.server_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(DeployableMissionList { missions }))
}

/// `POST /api/v1/game-runtime/deployments` — an in-game administrator's selection, relayed with
/// the Arma identity that made it. The identity must be linked to an unbanned platform
/// administrator, who becomes the requester; the selection is validated exactly as a website
/// request is.
///
/// @route POST /api/v1/game-runtime/deployments
pub async fn relayed_deployment_request(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Result<Json<RelayedDeploymentRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<MissionDeployment>), ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
    let mut transaction = state.pool.begin().await?;
    let deployment = request_deployment(
        &mut transaction,
        caller.server_id,
        request.mission_id,
        request.artifact_id,
        request.event_mission_id,
        Requester::InGame {
            arma_id: &request.requested_by_arma_id,
        },
        &state.cfg,
    )
    .await?;
    transaction.commit().await?;
    fail_point!(DeploymentRequestAfterCommit);
    Ok((StatusCode::ACCEPTED, Json(deployment)))
}
