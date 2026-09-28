//! The public catalog reads: the list of stored versions and one version's catalog document.
//!
//! **Role:** answers `GET /api/v1/ballistics-catalogs` with every stored version's summary and
//! `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}` with that version's catalog
//! document.
//!
//! **Position:** registered by [`crate::operations::routes::routes`] with no identity extractor;
//! reads through [`crate::operations::services::ballistics_catalogs::catalog_store`]. The mortar
//! calculator and the offline service worker are the callers.
//!
//! **Signals & state:** none.
//!
//! **Invariants:**
//! - A stored version never changes, so the document answer carries the strong ETag
//!   `"<catalog_sha256>"` and `Cache-Control: public, max-age=31536000, immutable`, and a request
//!   whose `If-None-Match` names that ETag (or `*`) answers 304 with the same two headers.
//! - An unknown catalog or version answers 404; a version segment that is not an integer 400.
//!
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalog
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogList

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::operations::models::ballistics_catalog::BallisticsCatalogList;
use crate::operations::services::ballistics_catalogs::catalog_store::{
    find_catalog_document, list_catalog_summaries,
};

/// The `Cache-Control` of a stored catalog version: cacheable by anyone, for a year, never
/// revalidated.
pub const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// `GET /api/v1/ballistics-catalogs`: every stored catalog version, ordered by catalog then
/// version.
///
/// @route GET /api/v1/ballistics-catalogs
pub async fn list_catalogs(
    State(state): State<AppState>,
) -> Result<Json<BallisticsCatalogList>, ApiError> {
    let data = list_catalog_summaries(&state.pool).await?;
    Ok(Json(BallisticsCatalogList { data }))
}

/// `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`: one stored catalog document.
///
/// @route GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}
pub async fn get_catalog_version(
    State(state): State<AppState>,
    PathParams((catalog_id, catalog_version)): PathParams<(String, i32)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let stored = find_catalog_document(&state.pool, &catalog_id, catalog_version)
        .await?
        .ok_or_else(|| ApiError::not_found("ballistics catalog version not found"))?;
    let etag = HeaderValue::from_str(&format!("\"{}\"", stored.catalog_sha256))
        .map_err(|_| ApiError::internal("internal error"))?;
    let cache_control = HeaderValue::from_static(IMMUTABLE_CACHE_CONTROL);
    if if_none_match_names(&headers, &etag) {
        return Ok((
            StatusCode::NOT_MODIFIED,
            [(header::ETAG, etag), (header::CACHE_CONTROL, cache_control)],
        )
            .into_response());
    }
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            ),
            (header::ETAG, etag),
            (header::CACHE_CONTROL, cache_control),
        ],
        stored.catalog_document,
    )
        .into_response())
}

/// Whether the request's `If-None-Match` lists `etag` or `*`.
fn if_none_match_names(headers: &HeaderMap, etag: &HeaderValue) -> bool {
    let Some(Ok(listed)) = headers
        .get(header::IF_NONE_MATCH)
        .map(|value| value.to_str())
    else {
        return false;
    };
    let etag = etag.to_str().unwrap_or_default();
    listed
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == etag)
}
