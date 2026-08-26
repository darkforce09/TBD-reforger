//! Virtual Arsenal registry handlers — flat item catalog (T-068.2) + compat edge
//! graph (T-068.9). Modpack-scoped; weak ETags for cheap client revalidation.
//!
//! **T-427 — cold-open bounded fetches.** Optional `limit`/`offset` pagination on both
//! list endpoints (omitting `limit` keeps the legacy full dump for back-compat). Compat
//! accepts comma-separated `edge_type` filters and a `view=cargo_defaults` aggregate so
//! the editor first open does not pull ~20k raw edges / ~7MB in one shot.
//!
//! @contract registry-items.schema.json#/$defs/item (each `/registry` row in "data")
//! @contract registry-compat.schema.json#/$defs/edge (each `/registry/compat` row in "data")

use std::collections::BTreeMap;

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

/// Catalog / compat page size when the client asks for `limit` (T-427). Higher than the
/// shared `PageParams` max of 100 so the editor can assemble ~1.8k items in a few shots
/// without re-introducing an unbounded single response.
const REGISTRY_PAGE_MAX: i64 = 500;
const REGISTRY_PAGE_DEFAULT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct RegistryQuery {
    modpack: Option<String>,
    /// When set, the response is a page (`data` ≤ limit) and carries `total`/`limit`/`offset`.
    /// When omitted, the legacy full catalog is returned (T-068.2 / G3 back-compat).
    limit: Option<i64>,
    offset: Option<i64>,
}

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

/// Clamp optional limit/offset for registry catalog + compat list pages (T-427).
fn registry_page_bounds(limit: Option<i64>, offset: Option<i64>) -> Option<(i64, i64)> {
    let limit = limit?;
    let limit = if limit <= 0 {
        REGISTRY_PAGE_DEFAULT
    } else {
        limit.min(REGISTRY_PAGE_MAX)
    };
    let offset = offset.filter(|&n| n >= 0).unwrap_or(0);
    Some((limit, offset))
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
/// Mirrors `arsenal_rules::cargo_container_from_evidence` so the aggregated view matches the
/// client-side walk the editor used to run over the full edge dump.
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

const ITEMS_SELECT: &str = "SELECT id, modpack_id, resource_name, display_name, category, \
     COALESCE(icon_url, '') AS icon_url, kind, \
     \"abstract\", arsenal_type, weight_kg, volume_cm3, max_weight_kg, max_volume_cm3, addon, \
     variant_of, cargo_grid_w, cargo_grid_h, sort_order, \
     COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
     FROM registry_items WHERE modpack_id = $1 \
     ORDER BY sort_order ASC, display_name ASC";

const ITEMS_SELECT_PAGE: &str = "SELECT id, modpack_id, resource_name, display_name, category, \
     COALESCE(icon_url, '') AS icon_url, kind, \
     \"abstract\", arsenal_type, weight_kg, volume_cm3, max_weight_kg, max_volume_cm3, addon, \
     variant_of, cargo_grid_w, cargo_grid_h, sort_order, \
     COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
     FROM registry_items WHERE modpack_id = $1 \
     ORDER BY sort_order ASC, display_name ASC \
     LIMIT $2 OFFSET $3";

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

/// `GET /api/v1/registry?modpack=<uuid>&limit=&offset=` — a modpack's flat catalog with a weak
/// ETag (`If-None-Match` → 304). Missing `modpack` → the current modpack. With `limit`, the
/// response is a bounded page and includes `total`/`limit`/`offset` (T-427).
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
    let page = registry_page_bounds(q.limit, q.offset);
    let (items, total, page_meta): (Vec<RegistryItem>, Option<i64>, Option<(i64, i64)>) =
        if let Some((limit, offset)) = page {
            let total: i64 =
                sqlx::query_scalar("SELECT count(*) FROM registry_items WHERE modpack_id = $1")
                    .bind(mp.id)
                    .fetch_one(&state.pool)
                    .await?;
            let items: Vec<RegistryItem> = sqlx::query_as(ITEMS_SELECT_PAGE)
                .bind(mp.id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&state.pool)
                .await?;
            (items, Some(total), Some((limit, offset)))
        } else {
            let items: Vec<RegistryItem> = sqlx::query_as(ITEMS_SELECT)
                .bind(mp.id)
                .fetch_all(&state.pool)
                .await?;
            (items, None, None)
        };

    let max_updated: i64 = items
        .iter()
        .filter_map(|it| it.updated_at.timestamp_nanos_opt())
        .max()
        .unwrap_or(0);
    // Unpaginated: keep the pre-T-068.9 / pre-T-427 ETag shape (no discriminator).
    // Paginated: include page bounds so page N never 304s against page M's body.
    let etag = match page_meta {
        Some((limit, offset)) => {
            let n = total.unwrap_or(items.len() as i64);
            format!(
                "W/\"{}-{}-{}-page-{}-{}\"",
                mp.id, n, max_updated, limit, offset
            )
        }
        None => format!("W/\"{}-{}-{}\"", mp.id, items.len(), max_updated),
    };

    if if_none_match(&headers, &etag) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let mut body = json!({
        "data": items,
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

/// `GET /api/v1/registry/compat?modpack=<uuid>&edge_type=<type>[&limit=&offset=][&view=cargo_defaults]`
/// — a modpack's compatibility edge graph (T-150 export) with a weak ETag
/// (`If-None-Match` → 304). Missing `modpack` → the current modpack; optional
/// `edge_type` filters to one family or a comma-separated set. `view=cargo_defaults`
/// returns the aggregated character→cargo seed map instead of raw edges (T-427).
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

    // ── T-427 slim cargo seed view (no raw edge dump on the wire) ────────────
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
mod t427_registry_bounds {
    use super::*;
    use crate::models::RegistryCompatEdge;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn edge(from: &str, to: &str, evidence: &str, qty: i32) -> RegistryCompatEdge {
        RegistryCompatEdge {
            id: Uuid::nil(),
            modpack_id: Uuid::nil(),
            from_node: from.into(),
            to_node: to.into(),
            edge_type: "character_default_cargo".into(),
            evidence: evidence.into(),
            qty,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            updated_at: Utc.timestamp_opt(0, 0).unwrap(),
        }
    }

    #[test]
    fn page_bounds_clamp_and_default() {
        assert_eq!(registry_page_bounds(None, Some(10)), None);
        assert_eq!(registry_page_bounds(Some(500), Some(0)), Some((500, 0)));
        assert_eq!(
            registry_page_bounds(Some(9999), Some(-1)),
            Some((REGISTRY_PAGE_MAX, 0))
        );
        assert_eq!(
            registry_page_bounds(Some(0), None),
            Some((REGISTRY_PAGE_DEFAULT, 0))
        );
    }

    #[test]
    fn edge_types_split_trim_dedupe() {
        assert!(parse_edge_types(None).is_empty());
        assert_eq!(
            parse_edge_types(Some("optic_on_weapon, mag_in_weapon,optic_on_weapon")),
            vec!["optic_on_weapon".to_string(), "mag_in_weapon".to_string()]
        );
    }

    #[test]
    fn cargo_aggregate_sums_qty_and_maps_containers() {
        let edges = vec![
            edge("mag_a", "char_1", "TargetStorage=Vest/Slot", 2),
            edge("mag_a", "char_1", "TargetStorage=Vest/Slot", 1),
            edge("bandage", "char_1", "TargetStorage=Pants/Pockets", 1),
            edge("ignored", "char_1", "TargetStorage=Helmet/X", 1),
            // Wrong family must not appear.
            RegistryCompatEdge {
                edge_type: "mag_in_weapon".into(),
                ..edge("x", "y", "TargetStorage=Vest/Slot", 1)
            },
        ];
        let map = aggregate_cargo_defaults(&edges);
        let char_1 = map.get("char_1").unwrap().as_array().unwrap();
        assert_eq!(char_1.len(), 2, "helmet evidence skipped; vest+pants kept");
        // BTreeMap order: (container, item) — pants before vest.
        assert_eq!(char_1[0]["container"], "pants");
        assert_eq!(char_1[0]["item"], "bandage");
        assert_eq!(char_1[0]["qty"], 1);
        assert_eq!(char_1[1]["container"], "vest");
        assert_eq!(char_1[1]["item"], "mag_a");
        assert_eq!(char_1[1]["qty"], 3);
    }

    #[test]
    fn cargo_aggregate_is_strictly_smaller_than_raw_edge_walk_input() {
        // Class-R shape pin: N raw cargo edges → fewer aggregated rows (duplicates collapse).
        // Proves the slim view cannot re-expand to the full dump.
        let mut edges = Vec::new();
        for i in 0..100 {
            edges.push(edge(
                &format!("item_{}", i % 10),
                &format!("char_{}", i % 5),
                "TargetStorage=Backpack/Slot",
                1,
            ));
        }
        let map = aggregate_cargo_defaults(&edges);
        let rows: usize = map
            .values()
            .map(|v| v.as_array().map(|a| a.len()).unwrap_or(0))
            .sum();
        assert!(rows < edges.len(), "aggregation must collapse duplicates");
        assert_eq!(map.len(), 5);
        // CRT: each char only pairs with items sharing the same mod-5 residue → 2 items/char.
        assert_eq!(rows, 10);
    }
}
