//! CMS — announcements CRUD + Discord push + image upload. Rust port of `handlers/cms.go`.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::field_tools::UPLOAD_DIR;
use crate::handlers::username;
use crate::middleware::AdminUser;
use crate::models::{Announcement, AnnouncementStatus, AnnouncementTag, AuditSeverity};
use crate::services::text::is_http_url;
use crate::services::{sanitize_html, snippet, write_audit};
use crate::state::AppState;

const MAX_UPLOAD_BYTES: usize = 5 << 20;

/// `announcements.thumbnail_url`, validated at the write boundary. **T-405**, adopting T-391's
/// guard on the second of the five URL columns that share its absent check.
///
/// The sink here is an `<img src>` (`frontend/src/announcements.rs`), which is weaker than the
/// `<a href>` that made T-391 a live XSS — browsers do not execute `javascript:` in `img src`.
/// The guard is the same anyway, for two reasons. Nothing had ever looked at this column, so
/// `javascript:`, `data:text/html,…` and `file:///…` all stored cleanly. And "weaker sink" is a
/// property of today's renderer, not of the column: the stored value is equally available to a CSS
/// `url()`, the Discord webhook two functions below, a CSV export, or a page nobody has written
/// yet — and every one of those would otherwise have to remember independently, forever.
///
/// **`""` passes**, exactly as at [`crate::handlers::telemetry::upsert_match`]. Absent-or-blank is
/// this column's "no thumbnail", it carries no scheme and cannot execute, and 400-ing it would
/// break a working shape to buy nothing. Trimmed first so the bytes validated are the bytes
/// stored — the T-218 house pattern.
///
/// Shared by create and PATCH so the two cannot drift, which is the exact way `valid_tag`'s
/// neighbours went wrong before (see [`valid_announcement_status`]).
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
/// They *had* drifted: `create_announcement` derived a bool with `input.status == "published"`,
/// so every value it did not recognise silently became a **Draft** — `"archived"` and even
/// `"PUBLISHED"` among them, i.e. an admin asking to publish got a draft and a 201 saying it
/// worked. Measured on the pre-fix binary, all four of `"bogus"`, `"archived"`, `"PUBLISHED"`
/// and absent returned `201` with `status = draft`. `update_announcement` meanwhile rejected the
/// same strings with a 400. Create is the one that was wrong: an unrecognised status is a caller
/// mistake, and the handler that silently downgrades it is the one hiding the mistake.
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

fn snippet_from(explicit: &str, body: &str) -> String {
    if !explicit.is_empty() {
        explicit.to_string()
    } else {
        snippet(body, 200)
    }
}

/// Push an announcement to the webhook; record the result. Returns success.
async fn push_to_discord(state: &AppState, a: &Announcement) -> bool {
    match state.webhook.push_announcement(a).await {
        Ok(msg_id) => {
            let _ = sqlx::query(
                "UPDATE announcements SET pushed_to_discord = true, discord_message_id = $1 WHERE id = $2",
            )
            .bind(&msg_id)
            .bind(a.id)
            .execute(&state.pool)
            .await;
            true
        }
        Err(_) => {
            write_audit(
                &state.pool,
                AuditSeverity::Crit,
                None,
                "system",
                "webhook.push_failed",
                &format!(
                    "Webhook failed to push payload to Discord channel #announcements ('{}')",
                    a.title
                ),
                "announcement",
                &a.id.to_string(),
            )
            .await;
            false
        }
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
    // Discord at the bottom of this function. `sanitize_html("   ")` is `"   "`, so nothing
    // downstream catches it either. Measured on the pre-fix binary, `{"title":"   ", ...}`
    // returned 201 and stored a three-space title. Empty was already refused here, so unlike
    // `events.rs::check_name_override` there is no "" case to preserve — but the stored bytes
    // stay verbatim below for the same reason: a padded-but-real title renders fine today and
    // is matched by nothing, so canonicalising it would only break a working case.
    if input.title.trim().is_empty() || input.body.trim().is_empty() {
        return Err(ApiError::bad_request("title and body are required"));
    }
    let Some(tag) = valid_tag(&input.tag) else {
        return Err(ApiError::bad_request("invalid tag"));
    };
    // T-405 — see `validated_thumbnail`. Rejected before the INSERT, so a bad URL stores nothing.
    let thumbnail_url = validated_thumbnail(&input.thumbnail_url)?;
    let author = &admin.0.discord_id;
    // Sanitize author-supplied HTML before persist (no stored XSS).
    let body_html = sanitize_html(&input.body);
    let snip = snippet_from(&input.snippet, &body_html);
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
    .bind(&body_html)
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
    let name = username(&state.pool, author).await;
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
    // that empties them — and here **`""` is refused too**, not just whitespace. That differs
    // from `events.rs::check_name_override`, where `""` is a real instruction ("clear the
    // override"); an announcement has no title-less state to return to. Measured on the pre-fix
    // binary there was no guard here at all: `{"title":"   "}`, `{"body":"   "}` and
    // `{"title":""}` each returned 200 and overwrote the stored sentinel.
    for (field, value) in [("title", &input.title), ("body", &input.body)] {
        if let Some(v) = value
            && v.trim().is_empty()
        {
            return Err(ApiError::bad_request(format!("{field} must not be blank")));
        }
    }
    // **T-405.** Same window, same reason: validated up here so a rejected URL leaves the row
    // untouched instead of applying the caller's other field edits and then 400-ing. PATCH is the
    // back door that matters — the create path could be perfectly guarded and this one would still
    // put `javascript:` in the column. `None` means "field absent", which is not an edit.
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
    if let Some(b) = &input.body {
        qb.push(", body = ").push_bind(sanitize_html(b));
    }
    if let Some(s) = &input.snippet {
        qb.push(", snippet = ").push_bind(s.clone());
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

/// `POST /api/v1/cms/announcements/:id/push-discord` — manual (re)push.
///
/// @route POST /api/v1/cms/announcements/:id/push-discord
pub async fn push_announcement_discord(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    if !state.webhook.enabled() {
        return Err(ApiError::bad_request("discord webhook not configured"));
    }
    let Some(a) = reload(&state, id).await? else {
        return Err(ApiError::not_found("announcement not found"));
    };
    if !push_to_discord(&state, &a).await {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "webhook push failed",
        ));
    }
    Ok(Json(json!({ "pushed": true })))
}

/// `POST /api/v1/cms/uploads` — thumbnail upload (multipart "file").
///
/// @route POST /api/v1/cms/uploads
pub async fn upload_image(
    _a: AdminUser,
    mut mp: Multipart,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("file field required"))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("").to_string();
        let ext = ext_lower(&filename);
        let data = field
            .bytes()
            .await
            .map_err(|_| ApiError::bad_request("file field required"))?;
        if data.len() > MAX_UPLOAD_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file exceeds 5MB",
            ));
        }
        if !matches!(ext.as_str(), ".jpg" | ".jpeg" | ".png" | ".webp") {
            return Err(ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "only JPG, PNG, WEBP allowed",
            ));
        }
        std::fs::create_dir_all(UPLOAD_DIR)
            .map_err(|_| ApiError::internal("storage unavailable"))?;
        let name = format!("{}{ext}", Uuid::new_v4());
        std::fs::write(format!("{UPLOAD_DIR}/{name}"), &data)
            .map_err(|_| ApiError::internal("could not save file"))?;
        return Ok((
            StatusCode::CREATED,
            Json(json!({ "url": format!("/uploads/{name}") })),
        ));
    }
    Err(ApiError::bad_request("file field required"))
}

/// Lowercase file extension including the dot (`.jpg`), or empty.
fn ext_lower(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => format!(".{}", ext.to_lowercase()),
        _ => String::new(),
    }
}

/// Load one announcement by id (no soft-delete filter — matches Go's `First` on a
/// model without `DeletedAt`… announcements are archived, not soft-deleted here).
async fn reload(state: &AppState, id: Uuid) -> Result<Option<Announcement>, ApiError> {
    sqlx::query_as("SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(ApiError::from)
}
