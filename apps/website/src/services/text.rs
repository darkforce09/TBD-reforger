//! Text utilities — Rust port of `services/text.go`. HTML sanitize + snippet/truncate.

use std::sync::OnceLock;

/// Sanitize user-authored rich text (announcement bodies). Go used
/// `bluemonday.UGCPolicy()`; this uses `ammonia` — a **documented bounded deviation**
/// (gate G8): different engines, so output is not guaranteed byte-identical on edge
/// cases. The exact UGCPolicy→ammonia allowlist mapping + a golden-corpus + no-XSS
/// property test land with the differential harness. Built once.
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
mod tests {
    use super::*;

    #[test]
    fn snippet_collapses_and_truncates() {
        assert_eq!(snippet("  hello   world\n\tfoo ", 100), "hello world foo");
        assert_eq!(snippet("aaaaaa", 3), "aaa…");
    }

    #[test]
    fn cap_runes_respects_hard_cap() {
        assert_eq!(cap_runes("hello", 10), "hello");
        assert_eq!(cap_runes("hello", 3).chars().count(), 3); // "he…"
    }

    #[test]
    fn sanitize_strips_scripts() {
        let out = sanitize_html("<p>ok</p><script>alert(1)</script>");
        assert!(out.contains("ok"));
        assert!(!out.contains("<script"));
    }
}
