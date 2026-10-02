//! HTML sanitation and the plain-text preview helpers.
//!
//! **Announcement bodies are not sanitized.** The SPA renders `announcements.body` as a Leptos
//! **text** node (`announcements.rs` `{p.to_string()}`), never via `inner_html`, so there is no
//! live HTML sink for user content. Running [`sanitize_html`] before persist would buy zero XSS
//! defence and **double-escape** bare `<` / `&` (ammonia escapes, then Leptos escapes again →
//! authors see literal `a &lt; b`). CMS stores the body as authored plain text and Leptos owns
//! the single escape at render. [`sanitize_html`] exists for a field that really does render
//! HTML; do not call it on anything the SPA renders as text.

use std::sync::OnceLock;

/// HTML cleaner (ammonia). **Not** the announcement-body path — see the module header.
pub fn sanitize_html(body: &str) -> String {
    static CLEANER: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    CLEANER
        .get_or_init(ammonia::Builder::default)
        .clean(body)
        .to_string()
}

/// Short plain-ish preview: collapse whitespace then [`truncate`] to `n` runes.
pub fn snippet(body: &str, n: usize) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&collapsed, n)
}

/// Shorten to at most `n` runes, appending `…` when cut (may exceed `n` by one rune).
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let head: String = s.chars().take(n).collect();
    format!("{head}…")
}

/// Shorten so the result — ellipsis included — never exceeds `n` runes (hard caps).
pub fn cap_runes(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let head: String = s.chars().take(n - 1).collect();
    format!("{head}…")
}

#[cfg(test)]
#[path = "tests/html_sanitizer.rs"]
mod tests;
