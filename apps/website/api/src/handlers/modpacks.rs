//! Modpack read handlers — Rust port of `handlers/modpacks.go` (+ the shared
//! `loadCurrentModpack` from wiki.go).

use axum::extract::State;
use axum::response::Json;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::AuthUser;
use crate::models::{Modpack, ModpackMod};
use crate::state::AppState;

/// A modpack with its mod list embedded (Go struct embedding → serde flatten).
#[derive(Debug, Serialize)]
pub struct ModpackDto {
    #[serde(flatten)]
    pub modpack: Modpack,
    pub mods: Vec<ModpackMod>,
}

/// Load a modpack's mods (ordered) and wrap it as a DTO.
pub async fn with_mods(pool: &PgPool, modpack: Modpack) -> sqlx::Result<ModpackDto> {
    let mods: Vec<ModpackMod> = sqlx::query_as(
        "SELECT * FROM modpack_mods WHERE modpack_id = $1 \
         ORDER BY is_key_dependency DESC, sort_order ASC",
    )
    .bind(modpack.id)
    .fetch_all(pool)
    .await?;
    Ok(ModpackDto { modpack, mods })
}

/// The active (`is_current`) modpack as a DTO, or `None` if none configured.
/// Shared by the dashboard + modpack endpoints.
pub async fn load_current_modpack(pool: &PgPool) -> sqlx::Result<Option<ModpackDto>> {
    let mp: Option<Modpack> = sqlx::query_as("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE is_current = true")
        .fetch_optional(pool)
        .await?;
    match mp {
        Some(mp) => Ok(Some(with_mods(pool, mp).await?)),
        None => Ok(None),
    }
}

/// Load one modpack DTO by id (or `None`).
pub async fn load_modpack(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<ModpackDto>> {
    let mp: Option<Modpack> = sqlx::query_as("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match mp {
        Some(mp) => Ok(Some(with_mods(pool, mp).await?)),
        None => Ok(None),
    }
}

/// `GET /api/v1/modpacks` — every modpack with its mods (current first).
///
/// @route GET /api/v1/modpacks
pub async fn list_modpacks(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let packs: Vec<Modpack> =
        sqlx::query_as("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks ORDER BY is_current DESC, created_at DESC")
            .fetch_all(&state.pool)
            .await?;
    let mut out = Vec::with_capacity(packs.len());
    for mp in packs {
        out.push(with_mods(&state.pool, mp).await?);
    }
    Ok(Json(json!({ "data": out })))
}

/// `GET /api/v1/modpacks/current` — the active modpack.
///
/// @route GET /api/v1/modpacks/current
pub async fn get_current_modpack(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<ModpackDto>, ApiError> {
    load_current_modpack(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("no current modpack configured"))
}
