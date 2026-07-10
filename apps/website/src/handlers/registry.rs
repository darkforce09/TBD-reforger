//! Virtual Arsenal registry handlers — flat item catalog (T-068.2) + compat edge
//! graph (T-068.9). Modpack-scoped; weak ETags for cheap client revalidation.
//!
//! @contract registry-items.schema.json#/$defs/item (each `/registry` row in "data")
//! @contract registry-compat.schema.json#/$defs/edge (each `/registry/compat` row in "data")

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::MissionMakerUser;
use crate::models::{Modpack, RegistryCompatEdge, RegistryItem};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RegistryQuery {
    modpack: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryCompatQuery {
    modpack: Option<String>,
    edge_type: Option<String>,
}

/// Resolve the target modpack: explicit `?modpack=<uuid>` or the current one.
/// A malformed / unknown id maps to 404 (matches the Go handler).
async fn resolve_modpack(pool: &PgPool, modpack: Option<&str>) -> Result<Modpack, ApiError> {
    match modpack.filter(|s| !s.is_empty()) {
        Some(raw) => {
            let Ok(id) = Uuid::parse_str(raw) else {
                return Err(ApiError::not_found("modpack not found"));
            };
            sqlx::query_as::<_, Modpack>("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| ApiError::not_found("modpack not found"))
        }
        None => sqlx::query_as::<_, Modpack>("SELECT id, name, version, total_size_bytes, COALESCE(workshop_url, '') AS workshop_url, is_current, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM modpacks WHERE is_current = true")
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| ApiError::not_found("no current modpack configured")),
    }
}

/// Weak ETag over the result set: modpack + row count + newest `updated_at`
/// (nanos) + a query discriminator (so filtered and unfiltered responses can
/// never satisfy each other's `If-None-Match`).
fn weak_etag(modpack: Uuid, count: usize, max_updated_nanos: i64, discriminator: &str) -> String {
    format!("W/\"{modpack}-{count}-{max_updated_nanos}-{discriminator}\"")
}

/// 304 if the caller's `If-None-Match` equals the computed ETag.
fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag)
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
    let mp = resolve_modpack(&state.pool, q.modpack.as_deref()).await?;

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

    let max_updated: i64 = items
        .iter()
        .filter_map(|it| it.updated_at.timestamp_nanos_opt())
        .max()
        .unwrap_or(0);
    // Kept shape-compatible with the pre-T-068.9 ETag (no discriminator dimension).
    let etag = format!("W/\"{}-{}-{}\"", mp.id, items.len(), max_updated);

    if if_none_match(&headers, &etag) {
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

/// `GET /api/v1/registry/compat?modpack=<uuid>&edge_type=<type>` — a modpack's
/// compatibility edge graph (T-150 export) with a weak ETag (`If-None-Match` →
/// 304). Missing `modpack` → the current modpack; optional `edge_type` filters
/// to one edge family (plain-text match — new families need no code change).
///
/// @route GET /api/v1/registry/compat
pub async fn list_registry_compat(
    State(state): State<AppState>,
    _u: MissionMakerUser,
    headers: HeaderMap,
    Query(q): Query<RegistryCompatQuery>,
) -> Result<Response, ApiError> {
    let mp = resolve_modpack(&state.pool, q.modpack.as_deref()).await?;
    let edge_type = q.edge_type.as_deref().filter(|s| !s.is_empty());

    let edges: Vec<RegistryCompatEdge> = match edge_type {
        Some(ty) => {
            sqlx::query_as(
                "SELECT id, modpack_id, from_node, to_node, edge_type, \
                 COALESCE(evidence, '') AS evidence, created_at, updated_at \
                 FROM registry_compat WHERE modpack_id = $1 AND edge_type = $2 \
                 ORDER BY edge_type ASC, from_node ASC, to_node ASC",
            )
            .bind(mp.id)
            .bind(ty)
            .fetch_all(&state.pool)
            .await?
        }
        None => {
            sqlx::query_as(
                "SELECT id, modpack_id, from_node, to_node, edge_type, \
                 COALESCE(evidence, '') AS evidence, created_at, updated_at \
                 FROM registry_compat WHERE modpack_id = $1 \
                 ORDER BY edge_type ASC, from_node ASC, to_node ASC",
            )
            .bind(mp.id)
            .fetch_all(&state.pool)
            .await?
        }
    };

    let max_updated: i64 = edges
        .iter()
        .filter_map(|e| e.updated_at.timestamp_nanos_opt())
        .max()
        .unwrap_or(0);
    let etag = weak_etag(mp.id, edges.len(), max_updated, edge_type.unwrap_or("all"));

    if if_none_match(&headers, &etag) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let body = json!({
        "data": edges,
        "etag": etag,
        "modpack_id": mp.id,
        "modpack_version": mp.version,
    });
    Ok(([(header::ETAG, etag.clone())], Json(body)).into_response())
}
