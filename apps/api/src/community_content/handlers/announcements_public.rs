//! The member-facing announcement feed: the published list and one published row.
//!
//! **Role:** answers `GET /api/v1/announcements` (the published feed, paged) and
//! `GET /api/v1/announcements/{id}` (one published row).
//! **Position:** registered by [`crate::community_content::routes::routes`]; reads `announcements`
//! into [`Announcement`] rows for the command center feed.
//! **Signals & state:** none; one or two pool queries per request.
//! **Invariants:** only rows with `status = 'published'` and no `deleted_at` are read; the feed
//! orders pinned rows first, then by `published_at DESC, id DESC`, a total order, so paging never
//! repeats or skips a row; a `limit` or `offset` that is not an integer answers 400 in the
//! `{error, details?}` envelope through [`ApiError::from_query_rejection`]; a malformed id answers
//! 400 and an unknown or unpublished one 404.
//!
//! @contract announcement.schema.json#/definitions/AnnouncementPage

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::community_content::models::announcement::Announcement;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::http::path_parameters::PathParams;
use crate::core::middleware::AuthUser;

/// `GET /api/v1/announcements` — published feed, pinned first then newest; rows published at the
/// same instant order by `id DESC`, so every page boundary is stable. A `limit` or `offset` that is
/// not an integer answers 400 in the error envelope.
///
/// @route GET /api/v1/announcements
pub async fn list_announcements(
    State(state): State<AppState>,
    _u: AuthUser,
    page: Result<Query<PageParams>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(page) = page.map_err(|rejection| {
        ApiError::from_query_rejection(rejection, "announcement page query")
    })?;
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM announcements WHERE status = 'published' AND deleted_at IS NULL",
    )
    .fetch_one(&state.pool)
    .await?;
    let items: Vec<Announcement> = sqlx::query_as(
        "SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements WHERE status = 'published' AND deleted_at IS NULL \
         ORDER BY is_pinned DESC, published_at DESC, id DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({ "data": items, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// `GET /api/v1/announcements/:id` — one published announcement.
///
/// @route GET /api/v1/announcements/:id
pub async fn get_announcement(
    State(state): State<AppState>,
    _u: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<Announcement>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let a: Option<Announcement> = sqlx::query_as(
        "SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements WHERE id = $1 AND status = 'published' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    a.map(Json)
        .ok_or_else(|| ApiError::not_found("announcement not found"))
}
