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
        sqlx::query_as("SELECT id, slug, category, title, COALESCE(icon, '') AS icon, body_md, nav_order, updated_by, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM wiki_pages ORDER BY nav_order ASC, title ASC")
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
    let page: Option<WikiPage> = sqlx::query_as("SELECT id, slug, category, title, COALESCE(icon, '') AS icon, body_md, nav_order, updated_by, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM wiki_pages WHERE slug = $1")
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
        sqlx::query_as("SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, COALESCE(primary_threat, '') AS primary_threat, COALESCE(profile_image_url, '') AS profile_image_url FROM vehicle_databases ORDER BY name ASC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(json!({ "data": vehicles })))
}

/// Body for authoring a wiki page (admin).
///
/// **`icon` and `nav_order` are deliberately required — do not add `#[serde(default)]` to
/// them (T-319).** Their three siblings below default *and are guarded* at the emptiness
/// check in [`upsert_wiki_page`]; these two defaulted with nothing behind them, which is the
/// same shape as T-185 (`roles`) and T-218 (`reason`): the default is not "no data", it
/// decodes as an affirmative value and gets bound straight into the `ON CONFLICT DO UPDATE`.
///
/// This route is `PUT` — create *or replace* — so every write is a full overwrite of the
/// stored row. Omitting `nav_order` therefore did not mean "leave the ordering alone", it
/// wrote `0`, which sorts the page to the top of `ORDER BY nav_order ASC` and silently
/// reshuffles the whole SOP navigation for every reader. Measured on the dev fixture: a
/// body-only typo fix on `medical-sop` (`nav_order` 3, third in the nav) returned 200 and
/// moved it to first, above the Field Manual, and wiped its `medical_services` icon in the
/// same request.
///
/// Note the fix is *presence*, not non-emptiness — the two are different questions here and
/// only the first one is the bug. `icon = ""` is a real, live state (the seeded
/// `server-rules` page has no icon, and the model omits the key from its JSON when empty),
/// and `nav_order = 0` is a legitimate "put me first". Both must stay writable. What must
/// not stay writable is *silence*: an absent field is now a decode error, which the
/// extractor below maps to 400, so a caller states its intent or gets told.
#[derive(Debug, Deserialize)]
pub struct WikiInput {
    #[serde(default)]
    category: String,
    #[serde(default)]
    title: String,
    icon: String,
    #[serde(default)]
    body_md: String,
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
    // Names all five, because after T-319 all five must be *present* — an omitted `icon` or
    // `nav_order` lands here as a decode error, and a 400 that only lists the other three
    // sends the caller hunting for a field they already sent.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("category, title, icon, body_md and nav_order are required")
    })?;
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

    let page: WikiPage = sqlx::query_as("SELECT id, slug, category, title, COALESCE(icon, '') AS icon, body_md, nav_order, updated_by, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM wiki_pages WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(page))
}
