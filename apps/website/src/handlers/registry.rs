//! Virtual Arsenal registry handler — Rust port of `handlers/registry.go` (T-068).
//!
//! @contract registry-items.schema.json#/$defs/item (each row in "data")

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::MissionMakerUser;
use crate::models::{Modpack, RegistryItem};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RegistryQuery {
    modpack: Option<String>,
}

/// `GET /api/v1/registry?modpack=<uuid>` — a modpack's flat catalog with a weak ETag
/// (`If-None-Match` → 304). Missing `modpack` → the current modpack.
///
/// @route GET /api/v1/registry
pub async fn list_registry(
    State(state): State<AppState>,
    _u: MissionMakerUser,
    headers: HeaderMap,
    Query(q): Query<RegistryQuery>,
) -> Result<Response, ApiError> {
    // Resolve the modpack: explicit ?modpack= or the current one.
    let mp: Modpack = match q.modpack.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => {
            let Ok(id) = Uuid::parse_str(raw) else {
                return Err(ApiError::not_found("modpack not found"));
            };
            sqlx::query_as::<_, Modpack>("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.pool)
                .await?
                .ok_or_else(|| ApiError::not_found("modpack not found"))?
        }
        None => sqlx::query_as::<_, Modpack>("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE is_current = true")
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| ApiError::not_found("no current modpack configured"))?,
    };

    // COALESCE nullable columns to Go/GORM's zero-values (non-pointer fields read NULL
    // as "" / the zero time) — the dev seed (registry_dev.sql) leaves icon_url +
    // created_at + updated_at NULL, which a bare `SELECT *` can't decode into the model.
    let items: Vec<RegistryItem> = sqlx::query_as(
        "SELECT id, modpack_id, resource_name, display_name, category, \
         COALESCE(icon_url, '') AS icon_url, kind, sort_order, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM registry_items WHERE modpack_id = $1 \
         ORDER BY sort_order ASC, display_name ASC",
    )
    .bind(mp.id)
    .fetch_all(&state.pool)
    .await?;

    // Weak ETag: modpack_id + row count + newest updated_at (nanos).
    let max_updated: i64 = items
        .iter()
        .filter_map(|it| it.updated_at.timestamp_nanos_opt())
        .max()
        .unwrap_or(0);
    let etag = format!("W/\"{}-{}-{}\"", mp.id, items.len(), max_updated);

    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag.as_str())
    {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let body = json!({
        "data": items,
        "etag": etag,
        "modpack_id": mp.id,
        "modpack_version": mp.version,
    });
    Ok(([(header::ETAG, etag.clone())], Json(body)).into_response())
}
