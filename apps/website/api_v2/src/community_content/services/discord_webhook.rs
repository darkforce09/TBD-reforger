//! Announcement → Discord webhook. Posts an embed to the #announcements channel and
//! returns the created message id.

use std::borrow::Cow;
use std::sync::Once;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::community_content::models::announcement::{Announcement, AnnouncementTag};
use crate::core::http_client::retry_on_429::send_with_retry_on_429;
use crate::core::text::html_sanitizer::{cap_runes, truncate};

static TLS_INIT: Once = Once::new();
fn ensure_tls_provider() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Neutralise formula / control characters in Discord embed text fields.
///
/// Parallel to [`crate::administration::handlers::audit_logs`]'s `escape_csv_formula`, but Discord is
/// **not** a spreadsheet sink — a leading `'` would show as a literal apostrophe in the channel.
/// Instead:
/// 1. Strip ASCII control characters (NUL‥US, DEL) so they cannot break Discord JSON/markdown
///    rendering or ride into audit messages that interpolate the same title.
/// 2. Prefix a leading `=`, `+`, `-`, or `@` with U+200B (ZWSP) so copy-paste into Excel/Sheets
///    does not become a live formula, without a visible CSV apostrophe.
///
/// Applied at the webhook sink (`push_announcement`), not at CMS persist — the SPA still shows
/// the authored title; only the Discord embed is sanitised.
pub fn sanitize_discord_embed_field(s: &str) -> Cow<'_, str> {
    let needs_strip = s.chars().any(|c| c.is_ascii_control());
    let cleaned: Cow<'_, str> = if needs_strip {
        Cow::Owned(s.chars().filter(|c| !c.is_ascii_control()).collect())
    } else {
        Cow::Borrowed(s)
    };
    match cleaned.as_bytes().first() {
        Some(b'=' | b'+' | b'-' | b'@') => Cow::Owned(format!("\u{200B}{}", cleaned.as_ref())),
        _ => cleaned,
    }
}

/// Pushes announcement embeds to the Discord webhook (empty URL disables pushing).
#[derive(Clone)]
pub struct WebhookService {
    url: String,
    http: Client,
}

#[derive(Serialize)]
struct EmbedFooter {
    text: String,
}

#[derive(Serialize)]
struct Embed {
    title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    color: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    timestamp: String,
    footer: EmbedFooter,
}

#[derive(Serialize)]
struct WebhookPayload {
    username: String,
    embeds: Vec<Embed>,
}

#[derive(Deserialize, Default)]
struct WebhookResponse {
    #[serde(default)]
    id: String,
}

/// Embed sidebar color for a tag.
fn tag_color(tag: AnnouncementTag) -> i64 {
    match tag {
        AnnouncementTag::Important => 0x00F8_7171,
        AnnouncementTag::Event => 0x004D_8EFF,
        AnnouncementTag::ModpackUpdate => 0x007B_D0FF,
        AnnouncementTag::Update => 0x00AD_C6FF,
    }
}

impl WebhookService {
    /// Construct with the configured webhook URL (empty disables pushing).
    pub fn new(url: String) -> Self {
        ensure_tls_provider();
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("build reqwest client");
        Self { url, http }
    }

    /// True if a webhook URL is configured.
    pub fn enabled(&self) -> bool {
        !self.url.is_empty()
    }

    /// Post the announcement as an embed; return the created Discord message id
    /// (via `?wait=true`). Errors are for the caller to log as a CRIT audit.
    pub async fn push_announcement(&self, a: &Announcement) -> anyhow::Result<String> {
        if !self.enabled() {
            anyhow::bail!("webhook not configured");
        }
        // Sanitize before cap — formula/control must not survive the Discord sink.
        let description_raw = if a.snippet.is_empty() {
            truncate(&a.body, 500)
        } else {
            a.snippet.clone()
        };
        let description = sanitize_discord_embed_field(&description_raw).into_owned();
        let payload = WebhookPayload {
            username: "TBD Operations".to_string(),
            embeds: vec![Embed {
                // Discord hard-rejects over its field caps (title 256, footer 2048).
                // Formula/control sanitize on the user title, sibling of the CSV export escape.
                title: cap_runes(sanitize_discord_embed_field(&a.title).as_ref(), 256),
                description,
                color: tag_color(a.tag),
                timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                footer: EmbedFooter {
                    text: cap_runes(&format!("Category: {}", a.tag.as_str()), 2048),
                },
            }],
        };

        let url = if self.url.contains('?') {
            format!("{}&wait=true", self.url)
        } else {
            format!("{}?wait=true", self.url)
        };
        let buf = serde_json::to_vec(&payload)?;

        let resp = send_with_retry_on_429(|| {
            self.http
                .post(&url)
                .header("content-type", "application/json")
                .body(buf.clone())
        })
        .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body: String = resp
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(4096)
                .collect();
            anyhow::bail!("webhook push: status {status}: {body}");
        }
        let out: WebhookResponse = resp.json().await.unwrap_or_default();
        Ok(out.id)
    }
}

#[cfg(test)]
#[path = "tests/discord_webhook.rs"]
mod tests;
