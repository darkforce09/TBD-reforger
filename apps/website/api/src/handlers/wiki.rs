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
/// **The `slug` guard below is T-349, and it is `require`-and-refuse, not `trim`-and-store.**
/// `slug` is the unique key (`idx_wiki_pages_slug`) *and* the `ON CONFLICT` target, and until
/// T-349 it carried no guard of any kind — the same shape T-358 found in `handlers/factions.rs`,
/// where `POST "USA"` then `POST "USA "` both answered 201 because the byte strings differ.
///
/// Measured on the pre-fix binary: `PUT /wiki/t349-medical-sop` then
/// `PUT /wiki/t349-medical-sop%20` both answered **200** and inserted **two rows with two
/// different ids** — the unique index never fired, because to Postgres `'x'` and `'x '` are
/// simply not equal. `PUT /wiki/%20` and `PUT /wiki/%09` likewise answered 200 and created pages
/// whose entire slug is one space and one tab. All four then render as separate rows in the
/// `GET /wiki` SOP nav, while `GET /wiki/t349-medical-sop` reaches only the first — so the padded
/// twin is a nav entry no reader can open. Worse, this route is `PUT` (create *or replace*): an
/// author fixing a typo at a slug they pasted with a trailing space does not replace the page
/// they meant, they silently mint a second one, and the readers keep seeing the stale original.
///
/// **Why refuse rather than normalise.** Both were live options here, because — unlike T-343's
/// `orbat_reservations.squad` ↔ `orbat_slots.squad` and T-346's armory `faction` ↔
/// `orbat_slots.faction` — nothing in the repo joins `wiki_pages.slug` to another column. Its
/// only readers are [`get_wiki_page`] (`WHERE slug = $1`) and the `ON CONFLICT (slug)` below,
/// and *both* derive the value from the same URL path segment, so there is no cross-table
/// agreement to preserve. What settles it is what normalising would *do*: trimming the write key
/// would make `PUT /wiki/medical-sop%20` overwrite `medical-sop` — retargeting a **full-row
/// replace** onto a different page than the URL names. Turning a caller's typo into a silent
/// destructive overwrite of someone else's page is a worse outcome than a 400. Refusing also
/// touches **no read at all**, which is the only option structurally incapable of the T-343
/// hazard (a write-side trim disagreeing with a read-side that never got one).
///
/// **Both halves are needed** (T-358): the emptiness check alone would still admit
/// `PUT /wiki/x%20`, and a padding check alone would still admit `PUT /wiki/%09`, because a
/// tab-only slug is a *content* problem, not a *padding* problem. `PUT /wiki/` — a genuinely
/// empty segment — already 404s at the router and never reaches here; the check stays anyway so
/// a future route change cannot quietly reopen it.
///
/// Reads are deliberately left alone. A padded slug on `GET /wiki/:slug` is a 404, which is the
/// right answer, and any padded row already in a deployed database stays readable at the exact
/// bytes it was written with rather than becoming unreachable the moment this ships.
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
