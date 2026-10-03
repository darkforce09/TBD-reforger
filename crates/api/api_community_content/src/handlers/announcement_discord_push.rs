//! Pushing an announcement to the Discord #announcements channel: the shared push-and-record
//! step used by the CMS writers, and the dedicated manual (re)push route.
//!
//! @contract announcement.schema.json#/definitions/DiscordPushOutcome

use api_identifiers::AnnouncementId;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{Value, json};

use crate::handlers::announcements_admin::reload;
use crate::models::announcement::{Announcement, AnnouncementStatus, AnnouncementTag};
use api_audit_log::AuditSeverity;
use api_audit_log::audit_writer::write_audit;
use api_discord::discord_webhook::{WebhookAnnouncement, sanitize_discord_embed_field};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

/// The webhook input of `a`: its title, body and snippet, its tag as the footer category, and the
/// tag's sidebar colour.
pub fn webhook_announcement(a: &Announcement) -> WebhookAnnouncement<'_> {
    WebhookAnnouncement {
        title: &a.title,
        body: &a.body,
        snippet: &a.snippet,
        category: a.tag.as_str(),
        color: embed_color(a.tag),
    }
}

/// Embed sidebar colour for a tag, as `0xRRGGBB`.
fn embed_color(tag: AnnouncementTag) -> i64 {
    match tag {
        AnnouncementTag::Important => 0x00F8_7171,
        AnnouncementTag::Event => 0x004D_8EFF,
        AnnouncementTag::ModpackUpdate => 0x007B_D0FF,
        AnnouncementTag::Update => 0x00AD_C6FF,
    }
}

/// Push an announcement to the webhook; record the result. Returns success.
///
/// Discord embed sanitisation (formula prefixes / ASCII controls) lives in
/// [`sanitize_discord_embed_field`] / `WebhookService::push_announcement` — the CMS path does
/// not re-implement it. Failure audit messages still run the same helper so a control-char
/// title cannot ride into the audit log via this format string.
pub(super) async fn push_to_discord(state: &AppState, a: &Announcement) -> bool {
    let message = webhook_announcement(a);
    match state.webhook.push_announcement(&message).await {
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
            // Same sanitiser as the Discord sink — keep control/formula chars out of the audit.
            let safe_title = sanitize_discord_embed_field(&a.title);
            write_audit(
                &state.pool,
                AuditSeverity::Crit,
                None,
                "system",
                "webhook.push_failed",
                &format!(
                    "Webhook failed to push payload to Discord channel #announcements ('{}')",
                    safe_title
                ),
                "announcement",
                &a.id.to_string(),
            )
            .await;
            false
        }
    }
}

/// `POST /api/v1/cms/announcements/:id/push-discord` — manual (re)push.
///
/// @route POST /api/v1/cms/announcements/:id/push-discord
///
/// Refuses unless `status == published`, matching the create/PATCH paths that gate their own
/// Discord push on published — otherwise this dedicated route is the one way a draft or an
/// archived row can still be broadcast.
pub async fn push_announcement_discord(
    State(state): State<AppState>,
    _a: AdminUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(id) = id.parse::<AnnouncementId>() else {
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

#[cfg(test)]
#[path = "tests/announcement_discord_push.rs"]
mod tests;
