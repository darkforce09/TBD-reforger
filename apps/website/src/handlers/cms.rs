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
use crate::services::{sanitize_html, snippet, write_audit};
use crate::state::AppState;

const MAX_UPLOAD_BYTES: usize = 5 << 20;

fn valid_tag(s: &str) -> Option<AnnouncementTag> {
    match s {
        "" | "update" => Some(AnnouncementTag::Update),
        "event" => Some(AnnouncementTag::Event),
        "modpack_update" => Some(AnnouncementTag::ModpackUpdate),
        "important" => Some(AnnouncementTag::Important),
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
    if input.title.is_empty() || input.body.is_empty() {
        return Err(ApiError::bad_request("title and body are required"));
    }
    let Some(tag) = valid_tag(&input.tag) else {
        return Err(ApiError::bad_request("invalid tag"));
    };
    let author = &admin.0.discord_id;
    // Sanitize author-supplied HTML before persist (no stored XSS).
    let body_html = sanitize_html(&input.body);
    let snip = snippet_from(&input.snippet, &body_html);
    let published = input.status == "published";

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
    .bind(&input.thumbnail_url)
    .bind(author)
    .bind(input.is_pinned)
    .bind(if published {
        AnnouncementStatus::Published
    } else {
        AnnouncementStatus::Draft
    })
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
    if let Some(u) = &input.thumbnail_url {
        qb.push(", thumbnail_url = ").push_bind(u.clone());
    }
    if let Some(p) = input.is_pinned {
        qb.push(", is_pinned = ").push_bind(p);
    }
    let mut now_publishing = false;
    if let Some(s) = &input.status {
        let status = match s.as_str() {
            "draft" => AnnouncementStatus::Draft,
            "published" => AnnouncementStatus::Published,
            "archived" => AnnouncementStatus::Archived,
            _ => return Err(ApiError::bad_request("invalid status")),
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
