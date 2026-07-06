//! Wiki + vehicle read/author handlers — Rust port of `handlers/wiki.go`.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::{VehicleDatabase, WikiPage};
use crate::state::AppState;

/// `GET /api/v1/wiki` — SOP nav list.
///
/// @route GET /api/v1/wiki
pub async fn list_wiki(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let pages: Vec<WikiPage> =
        sqlx::query_as("SELECT * FROM wiki_pages ORDER BY nav_order ASC, title ASC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "data": pages })))
}

/// `GET /api/v1/wiki/:slug` — one SOP document.
///
/// @route GET /api/v1/wiki/:slug
pub async fn get_wiki_page(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<WikiPage>, ApiError> {
    let page: Option<WikiPage> = sqlx::query_as("SELECT * FROM wiki_pages WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await?;
    page.map(Json)
        .ok_or_else(|| ApiError::not_found("wiki page not found"))
}

/// `GET /api/v1/vehicle-database` — the Vehicle Database / IFF table.
///
/// @route GET /api/v1/vehicle-database
pub async fn list_vehicles(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let vehicles: Vec<VehicleDatabase> =
        sqlx::query_as("SELECT * FROM vehicle_databases ORDER BY name ASC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "data": vehicles })))
}

/// Body for authoring a wiki page (admin).
#[derive(Debug, Deserialize)]
pub struct WikiInput {
    #[serde(default)]
    category: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    body_md: String,
    #[serde(default)]
    nav_order: i64,
}

/// `PUT /api/v1/wiki/:slug` — create or replace a wiki page (admin).
///
/// @route PUT /api/v1/wiki/:slug
pub async fn upsert_wiki_page(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    body: Result<Json<WikiInput>, JsonRejection>,
) -> Result<Json<WikiPage>, ApiError> {
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("category, title and body_md are required"))?;
    if input.category.is_empty() || input.title.is_empty() || input.body_md.is_empty() {
        return Err(ApiError::bad_request(
            "category, title and body_md are required",
        ));
    }
    sqlx::query(
        "INSERT INTO wiki_pages (slug, category, title, icon, body_md, nav_order, updated_by, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, now()) \
         ON CONFLICT (slug) DO UPDATE SET category = EXCLUDED.category, title = EXCLUDED.title, \
          icon = EXCLUDED.icon, body_md = EXCLUDED.body_md, nav_order = EXCLUDED.nav_order, \
          updated_by = EXCLUDED.updated_by, updated_at = now()",
    )
    .bind(&slug)
    .bind(&input.category)
    .bind(&input.title)
    .bind(&input.icon)
    .bind(&input.body_md)
    .bind(input.nav_order)
    .bind(&admin.0.discord_id)
    .execute(&state.pool)
    .await?;

    let page: WikiPage = sqlx::query_as("SELECT * FROM wiki_pages WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(page))
}
