//! The CMS announcement surface: the admin master list plus create, partial edit, and archive.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::community_content::handlers::announcement_discord_push::push_to_discord;
use crate::community_content::models::announcement::{
    Announcement, AnnouncementStatus, AnnouncementTag,
};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AdminUser;
use crate::core::text::html_sanitizer::{cap_runes, snippet};
use crate::core::text::http_url_guard::is_http_url;

/// `announcements.thumbnail_url`, validated at the write boundary.
///
/// Today's sink is an `<img src>`, which is weaker than an `<a href>` — browsers do not execute
/// `javascript:` in `img src`. The guard holds anyway, because "weaker sink" is a property of the
/// current renderer, not of the column: the stored value is equally available to a CSS `url()`,
/// the Discord webhook, a CSV export, or a page nobody has written yet, and every one of those
/// would otherwise have to remember the check independently, forever.
///
/// **`""` passes.** Absent-or-blank is this column's "no thumbnail", it carries no scheme and
/// cannot execute, and 400-ing it would break a working shape to buy nothing. Trimmed first so
/// the bytes validated are the bytes stored.
///
/// Shared by create and PATCH so the two cannot drift.
fn validated_thumbnail(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "thumbnail_url must be an absolute http:// or https:// URL",
    ))
}

fn valid_tag(s: &str) -> Option<AnnouncementTag> {
    match s {
        "" | "update" => Some(AnnouncementTag::Update),
        "event" => Some(AnnouncementTag::Event),
        "modpack_update" => Some(AnnouncementTag::ModpackUpdate),
        "important" => Some(AnnouncementTag::Important),
        _ => None,
    }
}

/// The announcement status vocabulary, shared by create and PATCH so the two cannot drift.
///
/// An unrecognised status is a caller mistake, and a handler that silently downgrades it to a
/// draft hides the mistake behind a 201 that says the write worked. Deriving the flag from
/// `status == "published"` instead would do exactly that to `"archived"` and `"PUBLISHED"` alike,
/// so both writers route through this one vocabulary and 400 on anything outside it.
///
/// `""` is deliberately **not** in here. On create it means "field absent" (the input struct is
/// `#[serde(default)]`) and the caller maps it to `Draft`; on PATCH absence is `None`, so an
/// explicit `""` is a caller error and must keep its 400 rather than silently un-publishing a
/// live announcement. Contrast [`valid_tag`] above, where `""` legitimately means "default tag".
fn valid_announcement_status(s: &str) -> Option<AnnouncementStatus> {
    match s {
        "draft" => Some(AnnouncementStatus::Draft),
        "published" => Some(AnnouncementStatus::Published),
        "archived" => Some(AnnouncementStatus::Archived),
        _ => None,
    }
}

/// Build the stored `snippet` column. Always respects the **200-rune** cap, even when the caller
/// supplies `snippet` explicitly. Derived snippets collapse whitespace via [`snippet`]; explicit
/// ones only hard-cap, so intentional spacing in a hand-written teaser survives.
fn snippet_from(explicit: &str, body: &str) -> String {
    if !explicit.is_empty() {
        cap_runes(explicit, 200)
    } else {
        snippet(body, 200)
    }
}

#[derive(Debug, Deserialize)]
pub struct AnnouncementInput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    thumbnail_url: String,
    #[serde(default)]
    is_pinned: bool,
    #[serde(default)]
    status: String,
    #[serde(default)]
    push_to_discord: bool,
}

/// `GET /api/v1/cms/announcements` — admin CMS master list (drafts + published).
///
/// Public `GET /announcements` is published-only; the Content Manager needs drafts too.
/// Archived rows (soft-delete via DELETE) are omitted so the editor matches post-archive UI.
/// Envelope matches the platform list shape `{data,total,limit,offset}`.
///
/// @route GET /api/v1/cms/announcements
pub async fn list_cms_announcements(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM announcements \
         WHERE deleted_at IS NULL AND status IN ('draft', 'published')",
    )
    .fetch_one(&state.pool)
    .await?;
    let items: Vec<Announcement> = sqlx::query_as(concat!(
        "SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements ",
        "WHERE deleted_at IS NULL AND status IN ('draft', 'published') ",
        "ORDER BY is_pinned DESC, updated_at DESC LIMIT $1 OFFSET $2"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({ "data": items, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// `POST /api/v1/cms/announcements` — create draft/published (+ optional push).
///
/// @route POST /api/v1/cms/announcements
pub async fn create_announcement(
    State(state): State<AppState>,
    admin: AdminUser,
    body: Result<Json<AnnouncementInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Announcement>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("title and body are required"))?;
    // `trim()`, not bare `is_empty()`: a whitespace-only title or body is not content, and this
    // guard is the only thing standing between it and a **published** announcement pushed to
    // Discord at the bottom of this function. The stored bytes stay verbatim below, though — a
    // padded-but-real title renders fine and is matched by nothing, so canonicalising it would
    // only break a working case.
    if input.title.trim().is_empty() || input.body.trim().is_empty() {
        return Err(ApiError::bad_request("title and body are required"));
    }
    let Some(tag) = valid_tag(&input.tag) else {
        return Err(ApiError::bad_request("invalid tag"));
    };
    // See `validated_thumbnail`. Rejected before the INSERT, so a bad URL stores nothing.
    let thumbnail_url = validated_thumbnail(&input.thumbnail_url)?;
    let author = &admin.0.discord_id;
    // **Plain-text body contract.** The SPA renders body as a Leptos text node, not `inner_html`.
    // Do **not** ammonia-sanitize here: that HTML-escapes `<`/`&`, then Leptos escapes again, and
    // authors see literal `a &lt; b`. XSS for this field is the text escape at render; store the
    // authored bytes.
    let snip = snippet_from(&input.snippet, &input.body);
    // Absent (`#[serde(default)]` → `""`) is a Draft; anything else must be a status this
    // resource actually has, or the caller hears about it. See [`valid_announcement_status`].
    let status = if input.status.is_empty() {
        AnnouncementStatus::Draft
    } else {
        let Some(s) = valid_announcement_status(&input.status) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        s
    };
    let published = status == AnnouncementStatus::Published;

    let a: Announcement = sqlx::query_as(
        "INSERT INTO announcements \
         (title, body, snippet, tag, thumbnail_url, author_id, is_pinned, status, published_at, \
          pushed_to_discord, discord_message_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, '', now(), now()) RETURNING id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(&input.title)
    .bind(&input.body)
    .bind(&snip)
    .bind(tag)
    .bind(&thumbnail_url)
    .bind(author)
    .bind(input.is_pinned)
    .bind(status)
    .bind(if published {
        Some(chrono::Utc::now())
    } else {
        None
    })
    .fetch_one(&state.pool)
    .await?;

    let mut a = a;
    if published && input.push_to_discord {
        push_to_discord(&state, &a).await;
        a = reload(&state, a.id).await?.unwrap_or(a);
    }
    let name = actor_display_name(&state.pool, author).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(author),
        &name,
        "announcement.create",
        &format!("{name} created announcement '{}'", a.title),
        "announcement",
        &a.id.to_string(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(a)))
}

#[derive(Debug, Deserialize)]
pub struct AnnouncementUpdate {
    title: Option<String>,
    body: Option<String>,
    snippet: Option<String>,
    tag: Option<String>,
    thumbnail_url: Option<String>,
    is_pinned: Option<bool>,
    status: Option<String>,
    push_to_discord: Option<bool>,
}

/// `PATCH /api/v1/cms/announcements/:id` — partial edit (+ draft→published push).
///
/// @route PATCH /api/v1/cms/announcements/:id
pub async fn update_announcement(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<AnnouncementUpdate>, JsonRejection>,
) -> Result<Json<Announcement>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Some(existing) = reload(&state, id).await? else {
        return Err(ApiError::not_found("announcement not found"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    // Validated before the builder runs, so a rejected blank leaves the row entirely untouched
    // rather than applying the caller's other field edits.
    //
    // `create_announcement` requires both fields non-blank, so PATCH must not be the back door
    // that empties them — and here **`""` is refused too**, not just whitespace: an announcement
    // has no title-less state to return to, so `""` is never a real instruction on this column.
    for (field, value) in [("title", &input.title), ("body", &input.body)] {
        if let Some(v) = value
            && v.trim().is_empty()
        {
            return Err(ApiError::bad_request(format!("{field} must not be blank")));
        }
    }
    // Same window, same reason: validated up here so a rejected URL leaves the row untouched
    // instead of applying the caller's other field edits and then 400-ing. PATCH is the back door
    // that matters — the create path could be perfectly guarded and this one would still put
    // `javascript:` in the column. `None` means "field absent", which is not an edit.
    let thumbnail_url = input
        .thumbnail_url
        .as_deref()
        .map(validated_thumbnail)
        .transpose()?;

    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("UPDATE announcements SET updated_at = now()");
    if let Some(t) = &input.title {
        qb.push(", title = ").push_bind(t.clone());
    }
    // Body is stored as authored plain text (see create_announcement). When body changes and the
    // caller did not send a new snippet, **recompute the preview on write** so the list teaser
    // cannot contradict the article. Derive-on-write (not on read): the CMS list path is a cheap
    // SELECT over many rows and must not pay `snippet()` per row; the stored column is the list
    // contract.
    if let Some(b) = &input.body {
        qb.push(", body = ").push_bind(b.clone());
        if input.snippet.is_none() {
            qb.push(", snippet = ").push_bind(snippet_from("", b));
        }
    }
    if let Some(s) = &input.snippet {
        qb.push(", snippet = ").push_bind(snippet_from(s, ""));
    }
    if let Some(t) = &input.tag {
        let Some(tag) = valid_tag(t) else {
            return Err(ApiError::bad_request("invalid tag"));
        };
        qb.push(", tag = ").push_bind(tag);
    }
    if let Some(u) = &thumbnail_url {
        qb.push(", thumbnail_url = ").push_bind(u.clone());
    }
    if let Some(p) = input.is_pinned {
        qb.push(", is_pinned = ").push_bind(p);
    }
    let mut now_publishing = false;
    if let Some(s) = &input.status {
        let Some(status) = valid_announcement_status(s) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        qb.push(", status = ").push_bind(status);
        if status == AnnouncementStatus::Published && existing.published_at.is_none() {
            qb.push(", published_at = now()");
            now_publishing = true;
        }
    }
    qb.push(" WHERE id = ").push_bind(id);
    qb.build()
        .execute(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut a = reload(&state, id)
        .await?
        .ok_or_else(|| ApiError::internal("could not load announcement"))?;
    if input.push_to_discord == Some(true)
        && a.status == AnnouncementStatus::Published
        && (now_publishing || !a.pushed_to_discord)
    {
        push_to_discord(&state, &a).await;
        a = reload(&state, id).await?.unwrap_or(a);
    }
    Ok(Json(a))
}

/// `DELETE /api/v1/cms/announcements/:id` — archive (recoverable).
///
/// @route DELETE /api/v1/cms/announcements/:id
pub async fn delete_announcement(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let res = sqlx::query("UPDATE announcements SET status = 'archived' WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("announcement not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Load one announcement by id. No soft-delete-status filter beyond `deleted_at`: archived rows
/// stay readable to the writers, which is what makes an archive recoverable. Shared with
/// [`super::announcement_discord_push`], the other writer that needs the stored row.
pub(super) async fn reload(state: &AppState, id: Uuid) -> Result<Option<Announcement>, ApiError> {
    sqlx::query_as(concat!(
        "SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements ",
        "WHERE id = $1 AND deleted_at IS NULL"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(ApiError::from)
}

#[cfg(test)]
#[path = "tests/announcements_admin.rs"]
mod tests;
