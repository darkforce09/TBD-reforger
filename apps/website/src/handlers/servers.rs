//! Server Intel read handlers — Rust port of `handlers/servers.go`.

use axum::extract::{Path, State};
use axum::response::Json;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::modpacks::{ModpackDto, load_modpack};
use crate::middleware::AuthUser;
use crate::models::{Server, ServerStatus};
use crate::state::AppState;

// Queries cast `inet`→text (`ip::text`) and `numeric`→f64 (`server_fps::float8`).

/// Full Server Intel card: server config + live status + required modpack.
#[derive(Debug, Serialize)]
pub struct ServerIntelDto {
    #[serde(flatten)]
    pub server: Server,
    pub status: Option<ServerStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_modpack: Option<ModpackDto>,
}

/// Compose a server with its status + required modpack.
async fn server_intel(pool: &PgPool, server: Server) -> sqlx::Result<ServerIntelDto> {
    let status: Option<ServerStatus> =
        sqlx::query_as("SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, uptime_seconds, current_match_id, ingame_time, ingame_weather, updated_at FROM server_statuses WHERE server_id = $1")
            .bind(server.id)
            .fetch_optional(pool)
            .await?;
    let required_modpack = match server.required_modpack_id {
        Some(id) => load_modpack(pool, id).await.ok().flatten(),
        None => None,
    };
    Ok(ServerIntelDto {
        server,
        status,
        required_modpack,
    })
}

/// `GET /api/v1/servers` — all servers with status.
///
/// @route GET /api/v1/servers
pub async fn list_servers(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let servers: Vec<Server> =
        sqlx::query_as("SELECT id, name, host(ip) AS ip, port, required_modpack_id, is_active FROM servers ORDER BY name ASC")
            .fetch_all(&state.pool)
            .await?;
    let mut out = Vec::with_capacity(servers.len());
    for s in servers {
        out.push(server_intel(&state.pool, s).await?);
    }
    Ok(Json(json!({ "data": out })))
}

/// `GET /api/v1/servers/:id/status` — Server Intel card for one server.
///
/// @route GET /api/v1/servers/:id/status
pub async fn get_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ServerIntelDto>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let server: Option<Server> =
        sqlx::query_as("SELECT id, name, host(ip) AS ip, port, required_modpack_id, is_active FROM servers WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(server) = server else {
        return Err(ApiError::not_found("server not found"));
    };
    Ok(Json(server_intel(&state.pool, server).await?))
}
