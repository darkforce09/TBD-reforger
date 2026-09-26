//! Server Intel reads: the `GET` half of the server surface.
//!
//! One "intel card" is a `servers` row plus its live `server_statuses` row, the modpack that row
//! requires, and the theater of the match it is currently running. Both read paths compose the
//! same card, so the list endpoint and the single-server endpoint cannot drift apart.
//!
//! Route registration lives in [`super::super::routes`]; auth tier travels with the handler via
//! the [`AuthUser`] extractor each takes, so a registration typo cannot silently downgrade it.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, State};
use axum::response::Json;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::community_content::models::modpack::{Modpack, ModpackMod};
use crate::community_content::services::modpack_lookup::{ModpackDto, load_modpack};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, role_rank};
use crate::missions::models::mission::TerrainType;
use crate::server_infrastructure::models::server::{
    Server, ServerStatus, ServerStatusRow, server_status_columns,
};

// Queries cast `inet`→text (`ip::text`) and `numeric`→f64 (`server_fps::float8`).

/// The six-column projection every read and write of a `servers` row returns, so a created or
/// updated row is identical in shape to a `GET /servers` row and the SPA can reuse one DTO
/// (`frontend/src/dto.rs::ServerRowDto`) for all of them. `host(ip)` renders the `inet` as bare
/// text — the cast [`list_servers`] uses.
///
/// A `macro_rules!` and not a `const &str` on purpose: sqlx 0.9's `SqlSafeStr` bound accepts only
/// `&'static str`, so a `format!`ed query needs `AssertSqlSafe` and a hand-written injection audit
/// (see `events.rs::sql`). Expanding through `concat!` instead keeps every query a single string
/// **literal** — one source of truth for the projection, no runtime allocation, and no audit to get
/// wrong later.
macro_rules! server_cols {
    () => {
        "id, name, host(ip) AS ip, port, required_modpack_id, is_active"
    };
}
pub(super) use server_cols;

/// Full Server Intel card: server config + live status + required modpack + theater.
///
/// **`terrain` is join-sourced, not a `servers` column.** It comes from `matches.terrain` keyed by
/// `server_statuses.current_match_id`. Explicit `null` when there is no live match — same encoding
/// as `status`, never `skip_serializing_if` (an `Option` + skip field would round-trip
/// absent→None→absent and keep the R-api gate green over a fictional key).
#[derive(Debug, Serialize)]
pub struct ServerIntelDto {
    #[serde(flatten)]
    pub server: Server,
    pub status: Option<ServerStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_modpack: Option<ModpackDto>,
    /// Theater of the current match (`matches.terrain`), or JSON `null` when unmatched.
    pub terrain: Option<TerrainType>,
}

/// Status row plus optional match theater — one LEFT JOIN, both list and single-card paths.
#[derive(Debug, Clone, sqlx::FromRow)]
struct StatusWithTerrain {
    #[sqlx(flatten)]
    status: ServerStatusRow,
    terrain: Option<TerrainType>,
}

impl StatusWithTerrain {
    fn into_parts(self) -> (ServerStatus, Option<TerrainType>) {
        (ServerStatus::from(self.status), self.terrain)
    }
}

/// Shared SELECT: `server_statuses` LEFT JOIN `matches` for theater.
const SERVER_STATUS_SELECT_ONE: &str = concat!(
    "SELECT ",
    server_status_columns!(),
    ", m.terrain AS terrain FROM server_statuses s \
     LEFT JOIN matches m ON m.id = s.current_match_id WHERE s.server_id = $1"
);
const SERVER_STATUS_SELECT_ANY: &str = concat!(
    "SELECT ",
    server_status_columns!(),
    ", m.terrain AS terrain FROM server_statuses s \
     LEFT JOIN matches m ON m.id = s.current_match_id WHERE s.server_id = ANY($1)"
);

/// Whether `user` may see servers outside the configured fleet (inactive ones).
pub(crate) fn sees_inactive_servers(user: &AuthUser) -> bool {
    role_rank(&user.role) >= role_rank("admin")
}

/// Compose a server with its status + required modpack + match theater (single-card path).
pub(super) async fn server_intel(pool: &PgPool, server: Server) -> sqlx::Result<ServerIntelDto> {
    let joined: Option<StatusWithTerrain> = sqlx::query_as(SERVER_STATUS_SELECT_ONE)
        .bind(server.id)
        .fetch_optional(pool)
        .await?;
    let (status, terrain) = match joined {
        Some(row) => {
            let (status, terrain) = row.into_parts();
            (Some(status), terrain)
        }
        None => (None, None),
    };
    let required_modpack = match server.required_modpack_id {
        Some(id) => load_modpack(pool, id).await.ok().flatten(),
        None => None,
    };
    Ok(ServerIntelDto {
        server,
        status,
        required_modpack,
        terrain,
    })
}

/// Prefetch statuses + required modpacks for a server list in a constant number of
/// round-trips (≤4), not one card composition per row.
///
/// Measured shape (empty list short-circuits to 0 extra queries after the servers SELECT):
///   1. `server_statuses WHERE server_id = ANY($1)`
///   2. `modpacks WHERE id = ANY($1)` (skipped when no required_modpack_id)
///   3. `modpack_mods WHERE modpack_id = ANY($1)` (skipped when no required_modpack_id)
async fn servers_intel_batch(
    pool: &PgPool,
    servers: Vec<Server>,
) -> sqlx::Result<Vec<ServerIntelDto>> {
    if servers.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<Uuid> = servers.iter().map(|s| s.id).collect();
    let statuses: Vec<StatusWithTerrain> = sqlx::query_as(SERVER_STATUS_SELECT_ANY)
        .bind(&ids)
        .fetch_all(pool)
        .await?;
    let mut status_by_id: HashMap<Uuid, (ServerStatus, Option<TerrainType>)> = HashMap::new();
    for row in statuses {
        let server_id = row.status.server_id;
        status_by_id.insert(server_id, row.into_parts());
    }

    let mp_ids: Vec<Uuid> = servers
        .iter()
        .filter_map(|s| s.required_modpack_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Keyed raw rows (Clone) — assemble `ModpackDto` per server without needing Clone on the DTO.
    let mut pack_by_id: HashMap<Uuid, (Modpack, Vec<ModpackMod>)> = HashMap::new();
    if !mp_ids.is_empty() {
        let modpacks: Vec<Modpack> = sqlx::query_as(
            "SELECT id, name, version, total_size_bytes, \
             COALESCE(workshop_url, '') AS workshop_url, is_current, \
             COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
             FROM modpacks WHERE id = ANY($1)",
        )
        .bind(&mp_ids)
        .fetch_all(pool)
        .await?;
        let mods: Vec<ModpackMod> = sqlx::query_as(
            "SELECT id, modpack_id, name, is_key_dependency, sort_order, \
             COALESCE(workshop_id, '') AS workshop_id, COALESCE(mod_guid, '') AS mod_guid, \
             COALESCE(version, '') AS version \
             FROM modpack_mods WHERE modpack_id = ANY($1) \
             ORDER BY is_key_dependency DESC, sort_order ASC",
        )
        .bind(&mp_ids)
        .fetch_all(pool)
        .await?;
        let mut mods_by_mp: HashMap<Uuid, Vec<ModpackMod>> = HashMap::new();
        for m in mods {
            mods_by_mp.entry(m.modpack_id).or_default().push(m);
        }
        for mp in modpacks {
            let mods = mods_by_mp.remove(&mp.id).unwrap_or_default();
            pack_by_id.insert(mp.id, (mp, mods));
        }
    }

    Ok(servers
        .into_iter()
        .map(|server| {
            let required_modpack = server.required_modpack_id.and_then(|id| {
                pack_by_id.get(&id).map(|(mp, mods)| ModpackDto {
                    modpack: mp.clone(),
                    mods: mods.clone(),
                })
            });
            let (status, terrain) = match status_by_id.remove(&server.id) {
                Some((status, terrain)) => (Some(status), terrain),
                None => (None, None),
            };
            ServerIntelDto {
                server,
                status,
                required_modpack,
                terrain,
            }
        })
        .collect())
}

/// `GET /api/v1/servers` — the configured fleet (active servers) with status; administrators
/// also see inactive servers, marked by `is_active`.
///
/// @route GET /api/v1/servers
pub async fn list_servers(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let servers: Vec<Server> = sqlx::query_as(concat!(
        "SELECT ",
        server_cols!(),
        " FROM servers WHERE is_active OR $1 ORDER BY name ASC, id ASC"
    ))
    .bind(sees_inactive_servers(&user))
    .fetch_all(&state.pool)
    .await?;
    // Batched prefetch — composing one card per row here would be an N+1.
    let out = servers_intel_batch(&state.pool, servers).await?;
    Ok(Json(json!({ "data": out })))
}

/// `GET /api/v1/servers/:id/status` — Server Intel card for one server; an inactive server is
/// not found for anyone but an administrator.
///
/// @route GET /api/v1/servers/:id/status
pub async fn get_server_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ServerIntelDto>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let server: Option<Server> = sqlx::query_as(concat!(
        "SELECT ",
        server_cols!(),
        " FROM servers WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(server) = server.filter(|server| server.is_active || sees_inactive_servers(&user))
    else {
        return Err(ApiError::not_found("server not found"));
    };
    Ok(Json(server_intel(&state.pool, server).await?))
}

#[cfg(test)]
#[path = "tests/server_intel.rs"]
mod tests;
