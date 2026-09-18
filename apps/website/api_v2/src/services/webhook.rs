//! Announcement → Discord webhook — Rust port of `services/webhook.go`. Posts an
//! embed to the #announcements channel and returns the created message id.

use std::borrow::Cow;
use std::sync::Once;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::{Announcement, AnnouncementTag};
use crate::services::http_retry::send_with_retry_on_429;
use crate::services::text::{cap_runes, truncate};

static TLS_INIT: Once = Once::new();
fn ensure_tls_provider() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Neutralise formula / control characters in Discord embed text fields (T-498).
///
/// Parallel to audit [`crate::handlers::audit`]'s `escape_csv_formula` (T-408), but Discord is
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
        // T-498: sanitize before cap — formula/control must not survive the Discord sink.
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
                // T-498: formula/control sanitize on user title (sibling of T-408 CSV escape).
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
mod tests {
    use super::*;

    /// T-498 Class-R: formula-leading embed text must not leave Discord with a live first char.
    ///
    /// RED: delete the `Some(b'=' | …)` arm (or always return `Cow::Borrowed`) — first char
    /// stays `=`/`+`/`-`/`@` and `assert!(!…)` fails.
    #[test]
    fn sanitize_discord_embed_field_neutralises_formula_prefixes() {
        for dangerous in [
            "=cmd|'/C calc'!A0",
            "=HYPERLINK(\"http://evil\")",
            "+1+1",
            "-1+1",
            "@SUM(A1)",
        ] {
            let out = sanitize_discord_embed_field(dangerous);
            let first = out.chars().next().expect("non-empty");
            assert!(
                !matches!(first, '=' | '+' | '-' | '@'),
                "formula-leading {dangerous:?} must not keep live first char, got {out:?}"
            );
            assert!(
                out.ends_with(dangerous) || out.contains(dangerous),
                "payload text must survive after the neutraliser prefix, got {out:?}"
            );
        }
        // Safe cells pass through unchanged (including empty and leading digit/letter).
        assert_eq!(sanitize_discord_embed_field(""), "");
        assert_eq!(sanitize_discord_embed_field("Op Red Dawn"), "Op Red Dawn");
        assert_eq!(sanitize_discord_embed_field("9=ok"), "9=ok");
        // Unlike CSV escape, Discord must NOT show a leading apostrophe.
        assert!(!sanitize_discord_embed_field("=x").starts_with('\''));
    }

    /// T-498 Class-R: ASCII control characters must be stripped from embed fields.
    ///
    /// RED: drop the `is_ascii_control` filter — NUL/tab/CR survive and these asserts fire.
    #[test]
    fn sanitize_discord_embed_field_strips_ascii_controls() {
        let dirty = "hello\u{0}world\tline\r\nbreak";
        let out = sanitize_discord_embed_field(dirty);
        assert!(
            !out.chars().any(|c| c.is_ascii_control()),
            "ASCII controls must be gone, got {out:?}"
        );
        assert_eq!(out.as_ref(), "helloworldlinebreak");
    }

    /// T-498 Class-R: the live sink (`push_announcement` title + description arms) must call
    /// `sanitize_discord_embed_field` — a helper-only green with raw `cap_runes(&a.title)` is a
    /// false green (the T-408 residual this ticket closes).
    ///
    /// RED: restore `title: cap_runes(&a.title, 256)` without sanitize — window assert fails.
    #[test]
    fn push_announcement_sanitises_title_and_description_at_sink() {
        const SRC: &str = include_str!("webhook.rs");
        let prod = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("webhook.rs must have a #[cfg(test)] module");

        let start = prod
            .find("pub async fn push_announcement")
            .expect("push_announcement must exist");
        let after = &prod[start..];
        let end = after[1..]
            .find("\n    pub async fn ")
            .or_else(|| after[1..].find("\n}"))
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let fn_body = &after[..end];

        // Assembled so a bait comment cannot satisfy — call site must sit next to title bind.
        let sanitize = format!("{}{}", "sanitize_discord_", "embed_field");
        assert!(
            fn_body.contains(&sanitize),
            "push_announcement must call `{sanitize}` (perturbation: raw title into embed)"
        );

        let title_arm = fn_body.find("title:").expect("embed title: arm must exist");
        let title_win = &fn_body[title_arm..fn_body.len().min(title_arm + 120)];
        assert!(
            title_win.contains(&sanitize),
            "title arm must call `{sanitize}` in-window (not a distant comment):\n{title_win}"
        );

        // Description path: sanitize after snippet/body pick.
        assert!(
            fn_body.matches(&sanitize).count() >= 2,
            "title + description must both go through `{sanitize}` (count ≥ 2)"
        );
    }
}
