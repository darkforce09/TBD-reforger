//! Announcement read handlers — Rust port of `handlers/announcements.go`.

use axum::extract::{Path, Query, State};
use axum::response::Json;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::PageParams;
use crate::middleware::AuthUser;
use crate::models::Announcement;
use crate::state::AppState;

/// `GET /api/v1/announcements` — published feed, pinned first then newest.
///
/// @route GET /api/v1/announcements
pub async fn list_announcements(
    State(state): State<AppState>,
    _u: AuthUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM announcements WHERE status = 'published' AND deleted_at IS NULL",
    )
    .fetch_one(&state.pool)
    .await?;
    let items: Vec<Announcement> = sqlx::query_as(
        "SELECT * FROM announcements WHERE status = 'published' AND deleted_at IS NULL \
         ORDER BY is_pinned DESC, published_at DESC LIMIT $1 OFFSET $2",
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
    Path(id): Path<String>,
) -> Result<Json<Announcement>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let a: Option<Announcement> = sqlx::query_as(
        "SELECT * FROM announcements WHERE id = $1 AND status = 'published' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    a.map(Json)
        .ok_or_else(|| ApiError::not_found("announcement not found"))
}
