//! Administrator routes of the fleet command ledger: accept a command (202 with its receipt),
//! follow receipts, and cancel a command no executor has claimed. Each write locks the server,
//! reauthorizes the administrator on that transaction and audits in it.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, role_rank};
use crate::identity_and_access::services::session_authorization::authorize_on_connection;
use crate::server_infrastructure::models::fleet_command::{
    FleetCommandList, FleetCommandReceipt, FleetCommandRequest,
};
use crate::server_infrastructure::services::fleet_commands::command_ledger::{
    cancel_command, enqueue_command, list_receipts, load_receipt,
};

fn parse(raw: &str, what: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request(format!("invalid {what}")))
}

/// Lock the server, then confirm the caller is still an administrator after the lock wait.
/// Returns the administrator and whether the server is active.
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

/// @route POST /api/v1/servers/:id/commands
pub async fn request_server_command(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<FleetCommandRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<FleetCommandReceipt>), ApiError> {
    let server = parse(&id, "server id")?;
    let Json(request) = body.map_err(|rejection| {
        ApiError::bad_request(format!("invalid body: {}", rejection.body_text()))
    })?;
    let mut transaction = state.pool.begin().await?;
    let (actor, active) = lock_server_as_admin(&mut transaction, &state, &admin, server).await?;
    if !active {
        return Err(ApiError::conflict(
            "a deactivated server accepts no commands",
        ));
    }
    let receipt = enqueue_command(&mut transaction, server, &request, &actor).await?;
    transaction.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(receipt)))
}

#[derive(Debug, Deserialize)]
pub struct CommandPage {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// @route GET /api/v1/servers/:id/commands
pub async fn list_server_commands(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    page: Result<Query<CommandPage>, QueryRejection>,
) -> Result<Json<FleetCommandList>, ApiError> {
    let server = parse(&id, "server id")?;
    let Query(page) = page.map_err(|_| ApiError::bad_request("invalid limit or offset"))?;
    if !(1..=100).contains(&page.limit) || page.offset < 0 {
        return Err(ApiError::bad_request(
            "limit must be 1 to 100 and offset non-negative",
        ));
    }
    Ok(Json(FleetCommandList {
        items: list_receipts(&state.pool, server, page.limit, page.offset).await?,
    }))
}

/// @route GET /api/v1/servers/:id/commands/:commandId
pub async fn get_server_command(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path((id, command)): Path<(String, String)>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let (server, command) = (parse(&id, "server id")?, parse(&command, "command id")?);
    Ok(Json(load_receipt(&state.pool, server, command).await?))
}

/// @route POST /api/v1/servers/:id/commands/:commandId/cancel
pub async fn cancel_server_command(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, command)): Path<(String, String)>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let (server, command) = (parse(&id, "server id")?, parse(&command, "command id")?);
    let mut transaction = state.pool.begin().await?;
    // A deactivated server's queued commands stay cancellable.
    let (actor, _) = lock_server_as_admin(&mut transaction, &state, &admin, server).await?;
    let receipt = cancel_command(&mut transaction, server, command, &actor).await?;
    transaction.commit().await?;
    Ok(Json(receipt))
}
