//! Announcement → Discord webhook. Posts an embed to the #announcements channel and
//! returns the created message id.
//!
//! **Role:** builds the embed of one [`WebhookAnnouncement`], sanitises its text fields at the
//! sink, posts it with a bounded 429 retry and answers the created message id.
//! **Position:** `api_discord`; the application state holds one [`WebhookService`]; the
//! community content announcement push maps its announcement into a [`WebhookAnnouncement`] and
//! calls [`WebhookService::push_announcement`].
//! **Signals & state:** the `reqwest` client and its connection pool; the rustls ring provider is
//! installed once per process.
//! **Invariants:** the title and the description pass [`sanitize_discord_embed_field`] before
//! the caps Discord enforces (title 256, footer 2048 characters); an empty webhook URL disables
//! pushing; every failure is a typed [`crate::error::Error`].

use std::borrow::Cow;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::tls_crypto_provider::ensure_tls_provider;
use api_foundation::text::html_sanitizer::{cap_runes, truncate};
use api_http_layer::http_client::retry_on_429::send_with_retry_on_429;

/// Neutralise formula / control characters in Discord embed text fields.
///
/// Parallel to the audit log CSV export's `escape_csv_formula`, but Discord is
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

/// The announcement fields the webhook embed is built from, and nothing else.
#[derive(Clone, Copy, Debug)]
pub struct WebhookAnnouncement<'a> {
    /// The embed title, before sanitising and the 256-character cap.
    pub title: &'a str,
    /// The announcement body; its first 500 characters describe an announcement without a
    /// snippet.
    pub body: &'a str,
    /// The embed description, when not empty.
    pub snippet: &'a str,
    /// The category the footer names as `Category: <category>`.
    pub category: &'a str,
    /// The embed's sidebar colour, as `0xRRGGBB`.
    pub color: i64,
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
    pub async fn push_announcement(&self, a: &WebhookAnnouncement<'_>) -> Result<String, Error> {
        if !self.enabled() {
            return Err(Error::WebhookNotConfigured);
        }
        // Sanitize before cap — formula/control must not survive the Discord sink.
        let description_raw = if a.snippet.is_empty() {
            truncate(a.body, 500)
        } else {
            a.snippet.to_string()
        };
        let description = sanitize_discord_embed_field(&description_raw).into_owned();
        let payload = WebhookPayload {
            username: "TBD Operations".to_string(),
            embeds: vec![Embed {
                // Discord hard-rejects over its field caps (title 256, footer 2048).
                // Formula/control sanitize on the user title, sibling of the CSV export escape.
                title: cap_runes(sanitize_discord_embed_field(a.title).as_ref(), 256),
                description,
                color: a.color,
                timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                footer: EmbedFooter {
                    text: cap_runes(&format!("Category: {}", a.category), 2048),
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
            return Err(Error::WebhookStatus { status, body });
        }
        let out: WebhookResponse = resp.json().await.unwrap_or_default();
        Ok(out.id)
    }
}

#[cfg(test)]
#[path = "tests/discord_webhook.rs"]
mod tests;
