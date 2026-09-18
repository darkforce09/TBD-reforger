//! Modpack authoring: create, full replace, set-current, and delete.
//!
//! Nested mods are always written as a whole list rather than patched per row, so a pack and its
//! `game.mods[]` entries can never disagree about which mods the pack contains.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::community_content::models::modpack::Modpack;
use crate::community_content::services::modpack_lookup::{ModpackDto, modpack_cols, with_mods};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;

/// One mod in a create/replace body.
#[derive(Debug, Deserialize)]
pub struct ModInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_key_dependency: bool,
    #[serde(default)]
    pub sort_order: i64,
    /// Reforger `game.mods[].modId` (Workshop id).
    #[serde(default)]
    pub workshop_id: String,
    /// Local addon GUID (distinct from Workshop id — see STAGING-SERVER.md).
    #[serde(default)]
    pub mod_guid: String,
    /// Optional version pin for `game.mods[].version`.
    #[serde(default)]
    pub version: String,
}

/// Body for `POST /modpacks` and `PUT /modpacks/:id` (full replace of nested mods).
#[derive(Debug, Deserialize)]
pub struct ModpackInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub total_size_bytes: i64,
    #[serde(default)]
    pub workshop_url: String,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default)]
    pub mods: Vec<ModInput>,
}

fn validated_name(raw: &str) -> Result<String, ApiError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    Ok(name.to_string())
}

fn validated_version(raw: &str) -> Result<String, ApiError> {
    let version = raw.trim();
    if version.is_empty() {
        return Err(ApiError::bad_request("version is required"));
    }
    Ok(version.to_string())
}

fn validated_mod(
    raw: &ModInput,
    index: usize,
) -> Result<(String, String, String, String), ApiError> {
    let name = raw.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request(format!(
            "mods[{index}].name is required"
        )));
    }
    Ok((
        name.to_string(),
        raw.workshop_id.trim().to_string(),
        raw.mod_guid.trim().to_string(),
        raw.version.trim().to_string(),
    ))
}

fn body_error(e: JsonRejection) -> ApiError {
    ApiError::with_details(
        StatusCode::BAD_REQUEST,
        "invalid modpack payload (expected name, version, total_size_bytes, mods[])",
        json!({ "reason": e.body_text() }),
    )
}

/// Clear `is_current` on every pack except `keep` (or all packs when `keep` is None).
async fn clear_current(
    tx: &mut Transaction<'_, Postgres>,
    keep: Option<Uuid>,
) -> Result<(), ApiError> {
    match keep {
        Some(id) => {
            sqlx::query(
                "UPDATE modpacks SET is_current = false WHERE id <> $1 AND is_current = true",
            )
            .bind(id)
            .execute(&mut **tx)
            .await?;
        }
        None => {
            sqlx::query("UPDATE modpacks SET is_current = false WHERE is_current = true")
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

/// Replace the nested mod list for `modpack_id` (delete-all + insert). Caller owns the txn.
async fn replace_mods(
    tx: &mut Transaction<'_, Postgres>,
    modpack_id: Uuid,
    mods: &[ModInput],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM modpack_mods WHERE modpack_id = $1")
        .bind(modpack_id)
        .execute(&mut **tx)
        .await?;
    for (i, m) in mods.iter().enumerate() {
        let (name, workshop_id, mod_guid, version) = validated_mod(m, i)?;
        let sort = if m.sort_order == 0 {
            i as i64
        } else {
            m.sort_order
        };
        sqlx::query(
            "INSERT INTO modpack_mods \
             (modpack_id, name, is_key_dependency, sort_order, workshop_id, mod_guid, version) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(modpack_id)
        .bind(&name)
        .bind(m.is_key_dependency)
        .bind(sort)
        .bind(&workshop_id)
        .bind(&mod_guid)
        .bind(&version)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// `POST /api/v1/modpacks` — create a pack + nested mods (admin).
///
/// @route POST /api/v1/modpacks
pub async fn create_modpack(
    State(state): State<AppState>,
    admin: AdminUser,
    body: Result<Json<ModpackInput>, JsonRejection>,
) -> Result<(StatusCode, Json<ModpackDto>), ApiError> {
    let Json(input) = body.map_err(body_error)?;
    let name = validated_name(&input.name)?;
    let version = validated_version(&input.version)?;
    if input.total_size_bytes < 0 {
        return Err(ApiError::bad_request("total_size_bytes must be >= 0"));
    }
    // Validate mods before opening the txn so a bad name never leaves a half-row.
    for (i, m) in input.mods.iter().enumerate() {
        let _ = validated_mod(m, i)?;
    }

    let mut tx = state.pool.begin().await?;
    if input.is_current {
        clear_current(&mut tx, None).await?;
    }
    let pack: Modpack = sqlx::query_as(concat!(
        "INSERT INTO modpacks (name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ($1, $2, $3, $4, $5, now()) RETURNING ",
        modpack_cols!()
    ))
    .bind(&name)
    .bind(&version)
    .bind(input.total_size_bytes)
    .bind(input.workshop_url.trim())
    .bind(input.is_current)
    .fetch_one(&mut *tx)
    .await?;
    replace_mods(&mut tx, pack.id, &input.mods).await?;
    tx.commit().await?;

    let dto = with_mods(&state.pool, pack).await?;
    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "modpack.create",
        &format!(
            "{actor_name} created modpack '{}' v{} ({} mods)",
            dto.modpack.name,
            dto.modpack.version,
            dto.mods.len()
        ),
        "modpack",
        &dto.modpack.id.to_string(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(dto)))
}

/// `PUT /api/v1/modpacks/:id` — replace pack fields + nested mod list (admin).
///
/// Nested mods are **replaced** (delete-all + insert), not patched per-row — the SPA
/// editor always sends the full list on Save.
///
/// @route PUT /api/v1/modpacks/:id
pub async fn replace_modpack(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<ModpackInput>, JsonRejection>,
) -> Result<Json<ModpackDto>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Json(input) = body.map_err(body_error)?;
    let name = validated_name(&input.name)?;
    let version = validated_version(&input.version)?;
    if input.total_size_bytes < 0 {
        return Err(ApiError::bad_request("total_size_bytes must be >= 0"));
    }
    for (i, m) in input.mods.iter().enumerate() {
        let _ = validated_mod(m, i)?;
    }

    let mut tx = state.pool.begin().await?;
    if input.is_current {
        clear_current(&mut tx, Some(id)).await?;
    }
    let pack: Option<Modpack> = sqlx::query_as(concat!(
        "UPDATE modpacks SET name = $2, version = $3, total_size_bytes = $4, \
         workshop_url = $5, is_current = $6 \
         WHERE id = $1 RETURNING ",
        modpack_cols!()
    ))
    .bind(id)
    .bind(&name)
    .bind(&version)
    .bind(input.total_size_bytes)
    .bind(input.workshop_url.trim())
    .bind(input.is_current)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(pack) = pack else {
        return Err(ApiError::not_found("modpack not found"));
    };
    replace_mods(&mut tx, pack.id, &input.mods).await?;
    tx.commit().await?;

    let dto = with_mods(&state.pool, pack).await?;
    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "modpack.update",
        &format!(
            "{actor_name} updated modpack '{}' v{} ({} mods)",
            dto.modpack.name,
            dto.modpack.version,
            dto.mods.len()
        ),
        "modpack",
        &dto.modpack.id.to_string(),
    )
    .await;
    Ok(Json(dto))
}

/// `POST /api/v1/modpacks/:id/set-current` — mark this pack as the sole current (admin).
///
/// @route POST /api/v1/modpacks/:id/set-current
pub async fn set_current_modpack(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<ModpackDto>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let mut tx = state.pool.begin().await?;
    clear_current(&mut tx, Some(id)).await?;
    let pack: Option<Modpack> = sqlx::query_as(concat!(
        "UPDATE modpacks SET is_current = true WHERE id = $1 RETURNING ",
        modpack_cols!()
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(pack) = pack else {
        return Err(ApiError::not_found("modpack not found"));
    };
    tx.commit().await?;

    let dto = with_mods(&state.pool, pack).await?;
    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "modpack.set_current",
        &format!("{actor_name} set modpack '{}' as current", dto.modpack.name),
        "modpack",
        &dto.modpack.id.to_string(),
    )
    .await;
    Ok(Json(dto))
}

/// `DELETE /api/v1/modpacks/:id` — hard-delete pack + nested mods (admin).
///
/// Refuses with **409** when `registry_items`, `servers.required_modpack_id`, or
/// `events.modpack_id` still reference the pack — those tables have no ON DELETE
/// cascade (schema is FK-free), so a silent delete would orphan rows.
///
/// @route DELETE /api/v1/modpacks/:id
pub async fn delete_modpack(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };

    let refs: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT COUNT(*)::bigint FROM registry_items WHERE modpack_id = $1), \
           (SELECT COUNT(*)::bigint FROM servers WHERE required_modpack_id = $1), \
           (SELECT COUNT(*)::bigint FROM events WHERE modpack_id = $1)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if refs.0 + refs.1 + refs.2 > 0 {
        return Err(ApiError::conflict(format!(
            "modpack is still referenced (registry_items={}, servers={}, events={})",
            refs.0, refs.1, refs.2
        )));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM modpack_mods WHERE modpack_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let name: Option<String> =
        sqlx::query_scalar("DELETE FROM modpacks WHERE id = $1 RETURNING name")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(name) = name else {
        return Err(ApiError::not_found("modpack not found"));
    };
    tx.commit().await?;

    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "modpack.delete",
        &format!("{actor_name} deleted modpack '{name}'"),
        "modpack",
        &id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
