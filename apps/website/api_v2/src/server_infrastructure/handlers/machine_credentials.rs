//! Administrator routes for machine credentials: issue (the secret is shown once), list without
//! secrets, and revoke one credential independently of the server's others. Each write locks the
//! server row, reauthorizes the administrator on that connection and audits in the same
//! transaction.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, role_rank};
use crate::identity_and_access::services::session_authorization::authorize_on_connection;
use crate::server_infrastructure::models::machine_credential::{
    IssuedMachineCredential, MachineCredential, MachineCredentialIssue, MachineCredentialList,
    MachineCredentialRevocation,
};
use crate::server_infrastructure::services::machine_credentials::{
    issue_machine_credential, list_machine_credentials, revoke_machine_credential,
};

fn server_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request("invalid server id"))
}

/// Lock the server, then confirm the caller is still an administrator after the lock wait.
/// Returns whether the server is active.
async fn lock_server_as_admin(
    connection: &mut PgConnection,
    state: &AppState,
    admin: &AdminUser,
    server: Uuid,
) -> Result<(String, bool), ApiError> {
    let active: bool =
        sqlx::query_scalar("SELECT is_active FROM servers WHERE id = $1 FOR NO KEY UPDATE")
            .bind(server)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or_else(|| ApiError::not_found("server not found"))?;
    let actor = authorize_on_connection(connection, &state.cfg, &admin.0.session_claims).await?;
    if role_rank(&actor.role) < role_rank("admin") {
        return Err(ApiError::forbidden("insufficient role"));
    }
    Ok((actor.discord_id, active))
}

/// @route POST /api/v1/servers/:id/credentials
pub async fn issue_server_credential(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<MachineCredentialIssue>, JsonRejection>,
) -> Result<(StatusCode, Json<IssuedMachineCredential>), ApiError> {
    let server = server_id(&id)?;
    let Json(request) = body.map_err(|rejection| {
        ApiError::bad_request(format!("invalid body: {}", rejection.body_text()))
    })?;
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
    Path(id): Path<String>,
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
    Path((id, credential)): Path<(String, String)>,
    query: Result<Query<MachineCredentialRevocation>, QueryRejection>,
) -> Result<Json<MachineCredential>, ApiError> {
    let server = server_id(&id)?;
    let credential =
        Uuid::parse_str(&credential).map_err(|_| ApiError::bad_request("invalid credential id"))?;
    let Query(revocation) =
        query.map_err(|_| ApiError::bad_request("a reason query parameter is required"))?;
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
