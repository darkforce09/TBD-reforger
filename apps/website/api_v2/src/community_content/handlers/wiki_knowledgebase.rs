//! The doctrine knowledgebase: the SOP navigation list, one markdown page, and the admin
//! create-or-replace writer behind it.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::community_content::models::wiki::WikiPage;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AdminUser, AuthUser};

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

/// Body for authoring a wiki page (admin).
///
/// **`icon` and `nav_order` are deliberately required — do not add `#[serde(default)]` to
/// them.** Their three siblings default *and are guarded* at the emptiness check in
/// [`upsert_wiki_page`]; these two have nothing behind them, so a default is not "no data": it
/// decodes as an affirmative value and is bound straight into the `ON CONFLICT DO UPDATE`.
///
/// This route is `PUT` — create *or replace* — so every write is a full overwrite of the stored
/// row. Omitting `nav_order` would therefore not mean "leave the ordering alone", it would write
/// `0`, which sorts the page to the top of `ORDER BY nav_order ASC` and silently reshuffles the
/// whole SOP navigation for every reader; omitting `icon` would blank the page's icon in the same
/// request.
///
/// The requirement is *presence*, not non-emptiness — the two are different questions here and
/// only the first one is a hazard. `icon = ""` is a real, live state (a page may legitimately
/// have no icon, and the model omits the key from its JSON when empty), and `nav_order = 0` is a
/// legitimate "put me first". Both stay writable. What must not be writable is *silence*: an
/// absent field is a decode error, which the extractor below maps to 400, so a caller states its
/// intent or gets told.
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
/// **The `slug` guard below is `require`-and-refuse, not `trim`-and-store.** `slug` is the unique
/// key (`idx_wiki_pages_slug`) *and* the `ON CONFLICT` target, and to Postgres `'x'` and `'x '`
/// are simply not equal — so without the guard the unique index never fires on a padded twin.
/// `PUT /wiki/medical-sop` and `PUT /wiki/medical-sop%20` would insert two rows with two
/// different ids, both of which render as separate entries in the `GET /wiki` nav while
/// `GET /wiki/medical-sop` reaches only the first, leaving a nav entry no reader can open.
/// A whitespace-only slug (`%20`, `%09`) would likewise mint a page whose entire identity is one
/// blank character.
///
/// **Why refuse rather than normalise.** Nothing in the repo joins `wiki_pages.slug` to another
/// column — its only readers are [`get_wiki_page`] (`WHERE slug = $1`) and the `ON CONFLICT
/// (slug)` below, and *both* derive the value from the same URL path segment, so there is no
/// cross-table agreement to preserve. What settles it is what normalising would *do*: trimming
/// the write key would make `PUT /wiki/medical-sop%20` overwrite `medical-sop` — retargeting a
/// **full-row replace** onto a different page than the URL names. Turning a caller's typo into a
/// silent destructive overwrite of someone else's page is a worse outcome than a 400. Refusing
/// also touches **no read at all**, which is the only option structurally incapable of a
/// write-side trim disagreeing with a read-side that never got one.
///
/// **Both halves are needed**: the emptiness check alone would still admit `PUT /wiki/x%20`, and
/// a padding check alone would still admit `PUT /wiki/%09`, because a tab-only slug is a
/// *content* problem, not a *padding* problem. `PUT /wiki/` — a genuinely empty segment — 404s at
/// the router and never reaches here; the check stays anyway so a future route change cannot
/// quietly reopen it.
///
/// Reads are deliberately left alone. A padded slug on `GET /wiki/:slug` is a 404, which is the
/// right answer, and any padded row already in a deployed database stays readable at the exact
/// bytes it was written with rather than becoming unreachable.
///
/// @route PUT /api/v1/wiki/:slug
pub async fn upsert_wiki_page(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    body: Result<Json<WikiInput>, JsonRejection>,
) -> Result<Json<WikiPage>, ApiError> {
    // Checked before the body, because the slug is the identity of the resource being written:
    // a caller who addressed the wrong row needs to hear about the row, not about its contents.
    if slug.trim().is_empty() {
        return Err(ApiError::bad_request("slug is required"));
    }
    if slug != slug.trim() {
        return Err(ApiError::bad_request(
            "slug must not have leading or trailing whitespace",
        ));
    }
    // Names all five, because all five must be *present* — an omitted `icon` or `nav_order` lands
    // here as a decode error, and a 400 that only lists the other three sends the caller hunting
    // for a field they already sent.
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
