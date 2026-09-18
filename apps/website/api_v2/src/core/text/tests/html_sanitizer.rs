//! Unit coverage for the cleaner and the preview helpers, including the escaping behaviour that
//! is the reason announcement bodies never pass through the cleaner.

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

/// Golden: ammonia HTML-escapes bare `<` / `&`. This is exactly why CMS must not call
/// [`sanitize_html`] on announcement bodies that Leptos then text-escapes.
///
/// RED perturbation: flip the `assert_ne!` to `assert_eq!` (claim identity) — this test
/// goes red, proving the pin still observes ammonia's mutation rather than a no-op cleaner.
#[test]
fn sanitize_html_escapes_bare_angle_brackets_and_ampersands() {
    let authored = "Damage threshold: a < b & c > d";
    let cleaned = sanitize_html(authored);
    assert_ne!(
        cleaned, authored,
        "ammonia must mutate plain text containing < / & (else the double-escape diagnosis in \
         the module header is stale)"
    );
    assert!(
        cleaned.contains("&lt;") || cleaned.contains("&amp;"),
        "expected HTML entities in {cleaned:?}"
    );
    // Scripts still die — the cleaner is real HTML sanitation, just the wrong tool for
    // a text-rendered field.
    assert!(!sanitize_html("<script>alert(1)</script>").contains("<script"));
}

/// Round-trip pin for the text-field contract: snippet derivation must not introduce
/// HTML entities. RED: replace `snippet` with `sanitize_html` below — fails on `a < b`.
#[test]
fn snippet_preserves_bare_angle_brackets() {
    let authored = "a < b & c";
    let snip = snippet(authored, 200);
    assert_eq!(snip, authored);
    assert!(!snip.contains("&lt;"));
    assert!(!snip.contains("&amp;"));
}
