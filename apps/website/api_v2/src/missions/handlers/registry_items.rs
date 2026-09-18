//! The Virtual Arsenal flat item catalog — one modpack's registry rows, optionally paged, behind
//! a weak ETag for cheap client revalidation.
//!
//! The modpack resolution, ETag and paging helpers live here because both registry surfaces need
//! them; [`super::registry_compat_graph`] reads them from this module rather than restating them.
//!
//! @contract registry-items.schema.json#/$defs/item (each `/registry` row in "data")

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::community_content::models::modpack::Modpack;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::MissionMakerUser;
use crate::missions::models::registry::RegistryItem;

/// Catalog / compat page size when the client asks for `limit`. Higher than the shared
/// `PageParams` max of 100 so the editor can assemble ~1.8k items in a few shots without
/// re-introducing an unbounded single response.
pub(super) const REGISTRY_PAGE_MAX: i64 = 500;
pub(super) const REGISTRY_PAGE_DEFAULT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct RegistryQuery {
    modpack: Option<String>,
    /// When set, the response is a page (`data` ≤ limit) and carries `total`/`limit`/`offset`.
    /// When omitted, the full catalog is returned.
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Resolve the target modpack: explicit `?modpack=<uuid>` or the current one.
/// A malformed / unknown id maps to 404.
pub(super) async fn resolve_modpack(
    pool: &PgPool,
    modpack: Option<&str>,
) -> Result<Modpack, ApiError> {
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
pub(super) fn weak_etag(
    modpack: Uuid,
    count: usize,
    max_updated_nanos: i64,
    discriminator: &str,
) -> String {
    format!("W/\"{modpack}-{count}-{max_updated_nanos}-{discriminator}\"")
}

/// 304 if the caller's `If-None-Match` equals the computed ETag.
pub(super) fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag)
}

/// Clamp optional limit/offset for registry catalog + compat list pages.
pub(super) fn registry_page_bounds(limit: Option<i64>, offset: Option<i64>) -> Option<(i64, i64)> {
    let limit = limit?;
    let limit = if limit <= 0 {
        REGISTRY_PAGE_DEFAULT
    } else {
        limit.min(REGISTRY_PAGE_MAX)
    };
    let offset = offset.filter(|&n| n >= 0).unwrap_or(0);
    Some((limit, offset))
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

/// `GET /api/v1/registry?modpack=<uuid>&limit=&offset=` — a modpack's flat catalog with a weak
/// ETag (`If-None-Match` → 304). Missing `modpack` → the current modpack. With `limit`, the
/// response is a bounded page and includes `total`/`limit`/`offset`.
///
/// @route GET /api/v1/registry
pub async fn list_registry(
    State(state): State<AppState>,
    _u: MissionMakerUser,
    headers: HeaderMap,
    Query(q): Query<RegistryQuery>,
) -> Result<Response, ApiError> {
    let mp = resolve_modpack(&state.pool, q.modpack.as_deref()).await?;

    // COALESCE nullable columns to the model's zero values (non-`Option` fields read NULL as
    // "" / the zero time) — the dev seed (registry_dev.sql) leaves icon_url + created_at +
    // updated_at NULL, which a bare `SELECT *` cannot decode into the model.
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
    // Unpaginated: the plain three-part ETag shape, no discriminator.
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

#[cfg(test)]
#[path = "tests/registry_items.rs"]
mod tests;
