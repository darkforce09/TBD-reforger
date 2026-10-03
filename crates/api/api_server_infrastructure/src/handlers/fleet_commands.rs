//! Administrator routes of the fleet command ledger: accept a command (202 with its receipt),
//! follow receipts, and cancel a command no executor has claimed. Each write locks the server,
//! reauthorizes the administrator on that transaction and audits in it.

use api_identifiers::{DiscordUserId, FleetCommandId, ServerId};
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use fleet_wire_contract::operator_messages::{
    FleetCommandList, FleetCommandReceipt, FleetCommandRequest,
};
use serde::Deserialize;
use sqlx::PgConnection;

use crate::services::fleet_commands::command_ledger::{
    cancel_command, enqueue_command, list_receipts, load_receipt,
};
use api_caller_identity::session_authorization::authorize_on_connection;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::{AdminUser, role_rank};
use api_state::AppState;

fn parse<Id: std::str::FromStr>(raw: &str, what: &str) -> Result<Id, ApiError> {
    raw.parse()
        .map_err(|_| ApiError::bad_request(format!("invalid {what}")))
}

/// Lock the server, then confirm the caller is still an administrator after the lock wait.
/// Returns the administrator and whether the server is active.
async fn lock_server_as_admin(
    connection: &mut PgConnection,
    state: &AppState,
    admin: &AdminUser,
    server: ServerId,
) -> Result<(DiscordUserId, bool), ApiError> {
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
    PathParams(id): PathParams<String>,
    body: Result<Json<FleetCommandRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<FleetCommandReceipt>), ApiError> {
    let server: ServerId = parse(&id, "server id")?;
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
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

/// `GET /servers/{id}/commands` query: the page of receipts to return.
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

/// A server's command receipts, newest first, one page at a time; an unknown server answers 404,
/// and a query that does not decode answers 400 through [`ApiError::from_query_rejection`].
///
/// @route GET /api/v1/servers/:id/commands
pub async fn list_server_commands(
    State(state): State<AppState>,
    _admin: AdminUser,
    PathParams(id): PathParams<String>,
    page: Result<Query<CommandPage>, QueryRejection>,
) -> Result<Json<FleetCommandList>, ApiError> {
    let server: ServerId = parse(&id, "server id")?;
    let Query(page) = page.map_err(|rejection| {
        ApiError::from_query_rejection(rejection, "server command page query")
    })?;
    if !(1..=100).contains(&page.limit) || page.offset < 0 {
        return Err(ApiError::bad_request(
            "limit must be 1 to 100 and offset non-negative",
        ));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM servers WHERE id = $1)")
        .bind(server)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(ApiError::not_found("server not found"));
    }
    Ok(Json(FleetCommandList {
        items: list_receipts(&state.pool, server, page.limit, page.offset).await?,
    }))
}

/// @route GET /api/v1/servers/:id/commands/:commandId
pub async fn get_server_command(
    State(state): State<AppState>,
    _admin: AdminUser,
    PathParams((id, command)): PathParams<(String, String)>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let (server, command): (ServerId, FleetCommandId) =
        (parse(&id, "server id")?, parse(&command, "command id")?);
    Ok(Json(load_receipt(&state.pool, server, command).await?))
}

/// @route POST /api/v1/servers/:id/commands/:commandId/cancel
pub async fn cancel_server_command(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams((id, command)): PathParams<(String, String)>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let (server, command): (ServerId, FleetCommandId) =
        (parse(&id, "server id")?, parse(&command, "command id")?);
    let mut transaction = state.pool.begin().await?;
    // A deactivated server's queued commands stay cancellable.
    let (actor, _) = lock_server_as_admin(&mut transaction, &state, &admin, server).await?;
    let receipt = cancel_command(&mut transaction, server, command, &actor).await?;
    transaction.commit().await?;
    Ok(Json(receipt))
}
