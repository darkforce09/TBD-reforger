//! The wiki revision history reads: one page of a page's revisions, and one revision.
//!
//! **Role:** answers `GET /api/v1/wiki/{slug}/revisions` and
//! `GET /api/v1/wiki/{slug}/revisions/{revision}` for signed-in members.
//! **Position:** registered in [`crate::community_content::routes::routes`]; reads
//! `wiki_page_revisions` through [`super::page_store`] and parses a revision's markdown with
//! [`crate::community_content::services::wiki_markup::read_markup`].
//! **Signals & state:** none; at most three pool queries per request.
//! **Invariants:** the history is newest first; `page` defaults to [`DEFAULT_PAGE`] and
//! `per_page` to [`DEFAULT_PER_PAGE`], a `per_page` above [`MAX_PER_PAGE`] is served as
//! [`MAX_PER_PAGE`], and a value below 1 or not a number answers 400 (a query string that does not
//! decode answers through [`ApiError::from_query_rejection`]); a page past the end answers no
//! items and the real total; an unknown slug answers 404, and so does a revision number the page
//! does not hold, while a revision that is not a number answers 400.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::response::Json;
use serde::Deserialize;

use super::page_store;
use crate::community_content::models::wiki::{WikiRevision, WikiRevisionPage};
use crate::community_content::services::wiki_markup::read_markup;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;

/// The page served when `page` is absent.
pub const DEFAULT_PAGE: i64 = 1;
/// The page size served when `per_page` is absent.
pub const DEFAULT_PER_PAGE: i64 = 20;
/// The largest page size served; a larger `per_page` is clamped to it.
pub const MAX_PER_PAGE: i64 = 100;

/// The history's query string; a value that is not an integer refuses the whole query.
#[derive(Debug, Deserialize)]
pub struct RevisionPageQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

/// `GET /api/v1/wiki/{slug}/revisions` — one page of the page's revisions, newest first.
///
/// @route GET /api/v1/wiki/{slug}/revisions
pub async fn list_wiki_revisions(
    State(state): State<AppState>,
    _member: AuthUser,
    Path(slug): Path<String>,
    query: Result<Query<RevisionPageQuery>, QueryRejection>,
) -> Result<Json<WikiRevisionPage>, ApiError> {
    let Query(query) = query
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "revision history query"))?;
    let (page, per_page) = page_window(query.page, query.per_page)?;
    let page_id = page_store::page_id(&state.pool, &slug)
        .await?
        .ok_or_else(|| ApiError::not_found("wiki page not found"))?;
    let total = page_store::revision_count(&state.pool, page_id).await?;
    let offset = (page - 1).saturating_mul(per_page);
    let items = page_store::revision_summaries(&state.pool, page_id, per_page, offset).await?;
    Ok(Json(WikiRevisionPage {
        items,
        page,
        per_page,
        total,
    }))
}

/// `GET /api/v1/wiki/{slug}/revisions/{revision}` — the page as one revision saved it.
///
/// @route GET /api/v1/wiki/{slug}/revisions/{revision}
pub async fn get_wiki_revision(
    State(state): State<AppState>,
    _member: AuthUser,
    Path((slug, revision)): Path<(String, String)>,
) -> Result<Json<WikiRevision>, ApiError> {
    let revision: i64 = revision
        .parse()
        .map_err(|_| ApiError::bad_request("revision must be a whole number"))?;
    let not_found = || ApiError::not_found("wiki revision not found");
    let revision = i32::try_from(revision).map_err(|_| not_found())?;
    let row = page_store::revision_by_number(&state.pool, &slug, revision)
        .await?
        .ok_or_else(not_found)?;
    let blocks = read_markup(&row.body_md).blocks;
    Ok(Json(WikiRevision::new(row, blocks)))
}

/// The served `(page, per_page)` for the requested values, or the 400 a value below 1 answers.
pub fn page_window(page: Option<i64>, per_page: Option<i64>) -> Result<(i64, i64), ApiError> {
    let page = page.unwrap_or(DEFAULT_PAGE);
    let per_page = per_page.unwrap_or(DEFAULT_PER_PAGE);
    if page < 1 {
        return Err(ApiError::bad_request("page must be 1 or greater"));
    }
    if per_page < 1 {
        return Err(ApiError::bad_request("per_page must be 1 or greater"));
    }
    Ok((page, per_page.min(MAX_PER_PAGE)))
}
