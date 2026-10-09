//! Administrator routes for machine credentials: issue (the secret is shown once), list without
//! secrets, and revoke one credential independently of the server's others. Each write locks the
//! server row, reauthorizes the administrator on that connection and audits in the same
//! transaction.

use api_identifiers::{MachineCredentialId, ServerId};
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;

use super::server_administration_lock::lock_server_as_admin;
use crate::models::machine_credential::{
    IssuedMachineCredential, MachineCredential, MachineCredentialIssue, MachineCredentialList,
    MachineCredentialRevocation,
};
use crate::services::machine_credentials::{
    issue_machine_credential, list_machine_credentials, revoke_machine_credential,
};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

fn server_id(raw: &str) -> Result<ServerId, ApiError> {
    raw.parse()
        .map_err(|_| ApiError::bad_request("invalid server id"))
}

/// @route POST /api/v1/servers/:id/credentials
pub async fn issue_server_credential(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams(id): PathParams<String>,
    body: Result<Json<MachineCredentialIssue>, JsonRejection>,
) -> Result<(StatusCode, Json<IssuedMachineCredential>), ApiError> {
    let server = server_id(&id)?;
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
    let mut transaction = state.pool.begin().await?;
    let (actor, active) = lock_server_as_admin(&mut transaction, &state, &admin, server).await?;
    if !active {
        return Err(ApiError::conflict(
            "a deactivated server receives no credentials",
        ));
    }
    let issued = issue_machine_credential(
        &mut transaction,
        server,
        request.executor_kind,
        &request.label,
        &actor,
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(issued)))
}

/// @route GET /api/v1/servers/:id/credentials
pub async fn list_server_credentials(
    State(state): State<AppState>,
    _admin: AdminUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<MachineCredentialList>, ApiError> {
    let server = server_id(&id)?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM servers WHERE id = $1)")
        .bind(server)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(ApiError::not_found("server not found"));
    }
    Ok(Json(MachineCredentialList {
        items: list_machine_credentials(&state.pool, server).await?,
    }))
}

/// @route DELETE /api/v1/servers/:id/credentials/:credentialId
pub async fn revoke_server_credential(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams((id, credential)): PathParams<(String, String)>,
    query: Result<Query<MachineCredentialRevocation>, QueryRejection>,
) -> Result<Json<MachineCredential>, ApiError> {
    let server = server_id(&id)?;
    let credential = credential
        .parse::<MachineCredentialId>()
        .map_err(|_| ApiError::bad_request("invalid credential id"))?;
    let Query(revocation) = query.map_err(|rejection| {
        ApiError::from_query_rejection(rejection, "credential revocation query")
    })?;
    let mut transaction = state.pool.begin().await?;
    let (actor, _) = lock_server_as_admin(&mut transaction, &state, &admin, server).await?;
    let revoked = revoke_machine_credential(
        &mut transaction,
        server,
        credential,
        &actor,
        &revocation.reason,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(revoked))
}
