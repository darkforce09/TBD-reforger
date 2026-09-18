//! The Virtual Arsenal compatibility edge graph — which item fits which, per modpack.
//!
//! Raw edges are optionally filtered by family and paged; `view=cargo_defaults` collapses the
//! `character_default_cargo` family server-side into a per-character seed map, so the editor's
//! cold open does not pull ~20k raw edges / ~7 MB in one shot.
//!
//! Modpack resolution, the weak ETag and the page clamp are shared with
//! [`super::registry_items`].
//!
//! @contract registry-compat.schema.json#/$defs/edge (each `/registry/compat` row in "data")

use std::collections::BTreeMap;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::MissionMakerUser;
use crate::missions::models::registry::RegistryCompatEdge;

use super::registry_items::{if_none_match, registry_page_bounds, resolve_modpack, weak_etag};

#[derive(Debug, Deserialize)]
pub struct RegistryCompatQuery {
    modpack: Option<String>,
    /// Plain-text edge family, or a comma-separated list (`optic_on_weapon,mag_in_weapon`).
    edge_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    /// `cargo_defaults` → aggregated per-character cargo seed map (no raw edge walk on the client).
    view: Option<String>,
}

/// Split `edge_type` on commas; empty tokens dropped. Preserves order, dedupes.
fn parse_edge_types(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for part in raw.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        if !out.iter().any(|e| e == t) {
            out.push(t.to_string());
        }
    }
    out
}

/// Map `character_default_cargo` evidence (`TargetStorage=<path>`) → Arsenal container key.
/// Mirrors `arsenal::rules::cargo_container_from_evidence` so the aggregated view matches the
/// client-side walk the editor would otherwise run over the full edge dump.
fn cargo_container_from_evidence(evidence: &str) -> Option<&'static str> {
    let path = evidence.strip_prefix("TargetStorage=")?;
    let seg = path.split('/').next().unwrap_or("").to_ascii_lowercase();
    if seg.starts_with("pants") {
        Some("pants")
    } else if seg.starts_with("jacket") {
        Some("jacket")
    } else if seg.starts_with("vest") {
        Some("vest")
    } else if seg.starts_with("back") {
        Some("backpack")
    } else {
        None
    }
}

/// Aggregate raw `character_default_cargo` edges into `{character → [{container,item,qty}]}`.
fn aggregate_cargo_defaults(
    edges: &[RegistryCompatEdge],
) -> serde_json::Map<String, serde_json::Value> {
    let mut by_char: BTreeMap<String, BTreeMap<(String, String), i64>> = BTreeMap::new();
    for e in edges {
        if e.edge_type != "character_default_cargo" {
            continue;
        }
        let Some(container) = cargo_container_from_evidence(&e.evidence) else {
            continue;
        };
        *by_char
            .entry(e.to_node.clone())
            .or_default()
            .entry((container.to_string(), e.from_node.clone()))
            .or_insert(0) += i64::from(e.qty);
    }
    let mut out = serde_json::Map::new();
    for (character, rows) in by_char {
        let arr: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|((container, item), qty)| {
                json!({
                    "container": container,
                    "item": item,
                    "qty": qty,
                })
            })
            .collect();
        out.insert(character, serde_json::Value::Array(arr));
    }
    out
}

const EDGES_ALL: &str = "SELECT id, modpack_id, from_node, to_node, edge_type, \
     COALESCE(evidence, '') AS evidence, qty, created_at, updated_at \
     FROM registry_compat WHERE modpack_id = $1 \
     ORDER BY edge_type ASC, from_node ASC, to_node ASC";

const EDGES_ALL_PAGE: &str = "SELECT id, modpack_id, from_node, to_node, edge_type, \
     COALESCE(evidence, '') AS evidence, qty, created_at, updated_at \
     FROM registry_compat WHERE modpack_id = $1 \
     ORDER BY edge_type ASC, from_node ASC, to_node ASC \
     LIMIT $2 OFFSET $3";

const EDGES_TYPED: &str = "SELECT id, modpack_id, from_node, to_node, edge_type, \
     COALESCE(evidence, '') AS evidence, qty, created_at, updated_at \
     FROM registry_compat WHERE modpack_id = $1 AND edge_type = ANY($2) \
     ORDER BY edge_type ASC, from_node ASC, to_node ASC";

const EDGES_TYPED_PAGE: &str = "SELECT id, modpack_id, from_node, to_node, edge_type, \
     COALESCE(evidence, '') AS evidence, qty, created_at, updated_at \
     FROM registry_compat WHERE modpack_id = $1 AND edge_type = ANY($2) \
     ORDER BY edge_type ASC, from_node ASC, to_node ASC \
     LIMIT $3 OFFSET $4";

const EDGES_CARGO_DEFAULTS: &str = "SELECT id, modpack_id, from_node, to_node, edge_type, \
     COALESCE(evidence, '') AS evidence, qty, created_at, updated_at \
     FROM registry_compat WHERE modpack_id = $1 AND edge_type = 'character_default_cargo' \
     ORDER BY from_node ASC, to_node ASC";

/// `GET /api/v1/registry/compat?modpack=<uuid>&edge_type=<type>[&limit=&offset=][&view=cargo_defaults]`
/// — a modpack's compatibility edge graph with a weak ETag (`If-None-Match` → 304). Missing
/// `modpack` → the current modpack; optional `edge_type` filters to one family or a
/// comma-separated set. `view=cargo_defaults` returns the aggregated character→cargo seed map
/// instead of raw edges.
///
/// @route GET /api/v1/registry/compat
pub async fn list_registry_compat(
    State(state): State<AppState>,
    _u: MissionMakerUser,
    headers: HeaderMap,
    Query(q): Query<RegistryCompatQuery>,
) -> Result<Response, ApiError> {
    let mp = resolve_modpack(&state.pool, q.modpack.as_deref()).await?;
    let view = q.view.as_deref().map(str::trim).filter(|s| !s.is_empty());

    // ── Slim cargo seed view (no raw edge dump on the wire) ──────────────────
    if view == Some("cargo_defaults") {
        let edges: Vec<RegistryCompatEdge> = sqlx::query_as(EDGES_CARGO_DEFAULTS)
            .bind(mp.id)
            .fetch_all(&state.pool)
            .await?;

        let max_updated: i64 = edges
            .iter()
            .filter_map(|e| e.updated_at.timestamp_nanos_opt())
            .max()
            .unwrap_or(0);
        let etag = weak_etag(mp.id, edges.len(), max_updated, "view-cargo_defaults");
        if if_none_match(&headers, &etag) {
            return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
        }

        let data = aggregate_cargo_defaults(&edges);
        let body = json!({
            "view": "cargo_defaults",
            "data": data,
            "etag": etag,
            "modpack_id": mp.id,
            "modpack_version": mp.version,
            // Raw edge count before aggregation — proves the server collapsed the walk.
            "source_edge_count": edges.len(),
        });
        return Ok(([(header::ETAG, etag.clone())], Json(body)).into_response());
    }

    let edge_types = parse_edge_types(q.edge_type.as_deref());
    let page = registry_page_bounds(q.limit, q.offset);

    let (edges, total, page_meta): (Vec<RegistryCompatEdge>, Option<i64>, Option<(i64, i64)>) =
        match (edge_types.is_empty(), page) {
            (true, None) => {
                let edges: Vec<RegistryCompatEdge> = sqlx::query_as(EDGES_ALL)
                    .bind(mp.id)
                    .fetch_all(&state.pool)
                    .await?;
                (edges, None, None)
            }
            (true, Some((limit, offset))) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM registry_compat WHERE modpack_id = $1",
                )
                .bind(mp.id)
                .fetch_one(&state.pool)
                .await?;
                let edges: Vec<RegistryCompatEdge> = sqlx::query_as(EDGES_ALL_PAGE)
                    .bind(mp.id)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&state.pool)
                    .await?;
                (edges, Some(total), Some((limit, offset)))
            }
            (false, None) => {
                let edges: Vec<RegistryCompatEdge> = sqlx::query_as(EDGES_TYPED)
                    .bind(mp.id)
                    .bind(&edge_types)
                    .fetch_all(&state.pool)
                    .await?;
                (edges, None, None)
            }
            (false, Some((limit, offset))) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM registry_compat WHERE modpack_id = $1 AND edge_type = ANY($2)",
                )
                .bind(mp.id)
                .bind(&edge_types)
                .fetch_one(&state.pool)
                .await?;
                let edges: Vec<RegistryCompatEdge> = sqlx::query_as(EDGES_TYPED_PAGE)
                    .bind(mp.id)
                    .bind(&edge_types)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&state.pool)
                    .await?;
                (edges, Some(total), Some((limit, offset)))
            }
        };

    let max_updated: i64 = edges
        .iter()
        .filter_map(|e| e.updated_at.timestamp_nanos_opt())
        .max()
        .unwrap_or(0);
    let type_disc = if edge_types.is_empty() {
        "all".to_string()
    } else {
        edge_types.join(",")
    };
    let discriminator = match page_meta {
        Some((limit, offset)) => format!("{type_disc}-page-{limit}-{offset}"),
        None => type_disc,
    };
    // ETag count: prefer full-set total when paging so the catalog version is stable across pages.
    let etag_count = total.map(|t| t as usize).unwrap_or(edges.len());
    let etag = weak_etag(mp.id, etag_count, max_updated, &discriminator);

    if if_none_match(&headers, &etag) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let mut body = json!({
        "data": edges,
        "etag": etag,
        "modpack_id": mp.id,
        "modpack_version": mp.version,
    });
    if let (Some(total), Some((limit, offset))) = (total, page_meta) {
        let obj = body.as_object_mut().expect("object");
        obj.insert("total".into(), json!(total));
        obj.insert("limit".into(), json!(limit));
        obj.insert("offset".into(), json!(offset));
    }
    Ok(([(header::ETAG, etag.clone())], Json(body)).into_response())
}

#[cfg(test)]
#[path = "tests/registry_compat_graph.rs"]
mod tests;
