//! Administrator deployment routes: request a deployment of an approved artifact on a server,
//! observe the server's deployments with their outcomes, and cancel one whose fleet command no
//! executor has claimed. Every read settles the server's deployment in flight first, so what an
//! operator sees is current.

use api_identifiers::{MissionDeploymentId, ServerId};
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use std::str::FromStr;

use crate::models::mission_deployment::{
    DeploymentRequest, MissionDeployment, MissionDeploymentPage,
};
use crate::services::mission_deployments::deployment_reads::{list_deployments, load_deployment};
use crate::services::mission_deployments::deployment_requests::{
    Requester, cancel_deployment, request_deployment,
};
use crate::services::mission_deployments::deployment_settlement::lock_and_settle;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::pagination::PageParams;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

fn parse_id<Id: FromStr>(raw: &str, what: &str) -> Result<Id, ApiError> {
    raw.parse()
        .map_err(|_| ApiError::bad_request(format!("invalid {what} id")))
}

/// `POST /api/v1/servers/:id/deployments` — validate and record a deployment, and issue the
/// fleet command that performs it.
///
/// @route POST /api/v1/servers/:id/deployments
pub async fn request_server_deployment(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams(server): PathParams<String>,
    body: Result<Json<DeploymentRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<MissionDeployment>), ApiError> {
    let server: ServerId = parse_id(&server, "server")?;
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
    let mut transaction = state.pool.begin().await?;
    let deployment = request_deployment(
        &mut transaction,
        server,
        request.mission_id,
        request.artifact_id,
        request.event_mission_id,
        Requester::Administrator(&admin.0),
        &state.cfg,
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

/// `GET /api/v1/servers/:id/deployments` — the server's deployments, newest first.
///
/// @route GET /api/v1/servers/:id/deployments
pub async fn list_server_deployments(
    State(state): State<AppState>,
    _admin: AdminUser,
    PathParams(server): PathParams<String>,
    query: Result<Query<PageParams>, QueryRejection>,
) -> Result<Json<MissionDeploymentPage>, ApiError> {
    let Query(page) = query
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "deployment page query"))?;
    let server: ServerId = parse_id(&server, "server")?;
    let (limit, offset) = page.bounds();
    let mut transaction = state.pool.begin().await?;
    lock_and_settle(&mut transaction, server).await?;
    let page = list_deployments(&mut transaction, server, limit, offset).await?;
    transaction.commit().await?;
    Ok(Json(page))
}

/// `GET /api/v1/servers/:id/deployments/:deploymentId` — one deployment and its outcome.
///
/// @route GET /api/v1/servers/:id/deployments/:deploymentId
pub async fn get_server_deployment(
    State(state): State<AppState>,
    _admin: AdminUser,
    PathParams((server, deployment)): PathParams<(String, String)>,
) -> Result<Json<MissionDeployment>, ApiError> {
    let server: ServerId = parse_id(&server, "server")?;
    let deployment: MissionDeploymentId = parse_id(&deployment, "deployment")?;
    let mut transaction = state.pool.begin().await?;
    lock_and_settle(&mut transaction, server).await?;
    let deployment = load_deployment(&mut transaction, server, deployment).await?;
    transaction.commit().await?;
    Ok(Json(deployment))
}

/// `POST /api/v1/servers/:id/deployments/:deploymentId/cancel` — cancel a deployment whose
/// fleet command is still queued; a claimed command answers 409.
///
/// @route POST /api/v1/servers/:id/deployments/:deploymentId/cancel
pub async fn cancel_server_deployment(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams((server, deployment)): PathParams<(String, String)>,
) -> Result<Json<MissionDeployment>, ApiError> {
    let server: ServerId = parse_id(&server, "server")?;
    let deployment: MissionDeploymentId = parse_id(&deployment, "deployment")?;
    let mut transaction = state.pool.begin().await?;
    let cancelled =
        cancel_deployment(&mut transaction, server, deployment, &admin.0, &state.cfg).await?;
    transaction.commit().await?;
    Ok(Json(cancelled))
}
