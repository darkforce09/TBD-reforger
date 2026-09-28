//! The doctrine wiki reads: the navigation list and one article.
//!
//! **Role:** answers `GET /api/v1/wiki` and `GET /api/v1/wiki/{slug}` for signed-in members.
//! **Position:** registered in [`crate::community_content::routes::routes`]; reads
//! `wiki_pages` through [`super::page_store`] and parses the article's markdown with
//! [`crate::community_content::services::wiki_markup::read_markup`].
//! **Signals & state:** none; one pool query per request.
//! **Invariants:** the list is `{"data": [...]}` of summaries ordered by `nav_order`, then
//! `title`, then `slug`; a slug is matched byte for byte, so an unknown or padded slug answers
//! 404; the article's `blocks` are the safe blocks of its `body_md`, whatever that body holds.

use axum::extract::State;
use axum::response::Json;

use super::page_store;
use crate::community_content::models::wiki::{WikiArticle, WikiPageList};
use crate::community_content::services::wiki_markup::read_markup;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::core::middleware::AuthUser;

/// `GET /api/v1/wiki` — every page's summary, in navigation order.
///
/// @route GET /api/v1/wiki
pub async fn list_wiki(
    State(state): State<AppState>,
    _member: AuthUser,
) -> Result<Json<WikiPageList>, ApiError> {
    let data = page_store::page_summaries(&state.pool).await?;
    Ok(Json(WikiPageList { data }))
}

/// `GET /api/v1/wiki/{slug}` — one article with its parsed blocks.
///
/// @route GET /api/v1/wiki/{slug}
pub async fn get_wiki_page(
    State(state): State<AppState>,
    _member: AuthUser,
    PathParams(slug): PathParams<String>,
) -> Result<Json<WikiArticle>, ApiError> {
    let page = page_store::page_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| ApiError::not_found("wiki page not found"))?;
    let blocks = read_markup(&page.body_md).blocks;
    Ok(Json(WikiArticle::new(page, blocks)))
}
