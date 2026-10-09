use super::*;

/// formula-leading embed text must not leave Discord with a live first char.
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

/// ASCII control characters must be stripped from embed fields.
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
