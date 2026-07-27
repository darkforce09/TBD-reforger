//! CMS — announcements CRUD + Discord push + image upload. Rust port of `handlers/cms.go`.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::field_tools::UPLOAD_DIR;
use crate::handlers::{PageParams, username};
use crate::middleware::AdminUser;
use crate::models::{Announcement, AnnouncementStatus, AnnouncementTag, AuditSeverity};
use crate::services::text::{cap_runes, is_http_url};
use crate::services::{snippet, write_audit};
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

/// Build the stored `snippet` column. Always respects the **200-rune** cap — even when the
/// caller supplies `snippet` explicitly (pre-T-239 returned that value verbatim and skipped the
/// limit). Derived snippets collapse whitespace via [`snippet`]; explicit ones only hard-cap so
/// intentional spacing in a hand-written teaser survives.
fn snippet_from(explicit: &str, body: &str) -> String {
    if !explicit.is_empty() {
        cap_runes(explicit, 200)
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
    // Discord at the bottom of this function. Measured on the pre-fix binary, `{"title":"   ", ...}`
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
    // **T-239 — plain-text body contract.** The SPA renders body as a Leptos text node, not
    // `inner_html`. Do **not** ammonia-sanitize here: that HTML-escapes `<`/`&`, then Leptos
    // escapes again, and authors see literal `a &lt; b`. XSS for this field is the text escape
    // at render; store the authored bytes.
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
    // T-239: store body as authored plain text (see create_announcement). When body changes and
    // the caller did not send a new snippet, recompute the preview so the list teaser cannot
    // contradict the article (pre-T-239 left the old snippet in place on body-only PATCH).
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

/// `POST /api/v1/cms/announcements/:id/push-discord` — manual (re)push.
///
/// @route POST /api/v1/cms/announcements/:id/push-discord
///
/// **T-246** — refuse unless `status == published`. Create/PATCH already gate Discord
/// push on published; this dedicated route did not, so a draft or archived row could
/// be broadcast if the endpoint was hit (SPA-unreachable today, still a live hole).
pub async fn push_announcement_discord(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Some(a) = reload(&state, id).await? else {
        return Err(ApiError::not_found("announcement not found"));
    };
    if a.status != AnnouncementStatus::Published {
        return Err(ApiError::bad_request(
            "only published announcements can be pushed to Discord",
        ));
    }
    if !state.webhook.enabled() {
        return Err(ApiError::bad_request("discord webhook not configured"));
    }
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
mod tests {
    use super::*;

    #[test]
    fn snippet_from_caps_explicit_and_derives_from_body() {
        let long = "x".repeat(250);
        let capped = snippet_from(&long, "ignored");
        assert_eq!(capped.chars().count(), 200);
        assert!(capped.ends_with('…'));

        let derived = snippet_from("", "a < b & c");
        assert_eq!(derived, "a < b & c");
        assert!(!derived.contains("&lt;"));
    }

    /// T-447 / T-465 Class-R — CMS list must be AdminUser-gated and filter drafts+published
    /// on both the count and list SQL (not published-only; not a bait comment).
    ///
    /// RED perturbations (Wave 25 verifier):
    /// - B1: leave `status IN ('draft', 'published')` in a comment + published-only SQL → FAIL
    ///   (filter must sit in both `query_scalar` and `query_as` windows; count == 2).
    /// - M1: drop `_a: AdminUser` from `list_cms_announcements` → FAIL (handler-slice pin).
    #[test]
    fn list_cms_announcements_is_drafts_plus_published_not_public_feed() {
        const SRC: &str = include_str!("cms.rs");
        let prod = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("cms.rs must have a #[cfg(test)] module");

        let start = prod.find("pub async fn list_cms_announcements").expect(
            "CMS GET list handler must exist (perturbation: remove list_cms_announcements)",
        );
        let after = &prod[start..];
        // Next sibling `pub async fn` ends the handler (create_announcement follows today).
        let end = after[1..]
            .find("\npub async fn ")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let handler = &after[..end];

        // M1 — AdminUser on THIS handler (sibling extractors elsewhere must not satisfy).
        let admin_pin = format!("{}{}", "_a: ", "AdminUser");
        assert!(
            handler.contains(&admin_pin),
            "list_cms_announcements must take `{admin_pin}` (perturbation: remove AdminUser)"
        );

        // B1 — drafts+published filter on both count (`query_scalar`) and list (`query_as`).
        // Assembled so a free-floating bait comment / this test's source cannot false-green.
        let filter = format!("{}{}", "status IN ('draft', ", "'published')");
        assert_eq!(
            handler.matches(&filter).count(),
            2,
            "count + list SQL must each use `{filter}` (bait comment alone / published-only FAIL)"
        );

        let qs = handler
            .find("query_scalar")
            .expect("list_cms_announcements must use query_scalar for total");
        let qs_win = &handler[qs..handler.len().min(qs + 280)];
        assert!(
            qs_win.contains(&filter),
            "count query_scalar window must contain `{filter}`"
        );

        let qa = handler
            .find("query_as")
            .expect("list_cms_announcements must use query_as for rows");
        let qa_win = &handler[qa..handler.len().min(qa + 520)];
        assert!(
            qa_win.contains(&filter),
            "list query_as window must contain `{filter}`"
        );

        const APP: &str = include_str!("../app.rs");
        assert!(
            APP.contains(
                "get(handlers::cms::list_cms_announcements).post(handlers::cms::create_announcement)"
            ),
            "app.rs must MethodRouter GET+POST /cms/announcements"
        );
    }
}
