use super::*;

/// Class-R: formula-leading embed text must not leave Discord with a live first char.
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

/// Class-R: ASCII control characters must be stripped from embed fields.
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

/// Class-R: the live sink (`push_announcement` title + description arms) must call
/// `sanitize_discord_embed_field` — a helper-only green with raw `cap_runes(&a.title)` is a
/// false green.
///
/// RED: restore `title: cap_runes(&a.title, 256)` without sanitize — window assert fails.
#[test]
fn push_announcement_sanitises_title_and_description_at_sink() {
    const SRC: &str = include_str!("../discord_webhook.rs");
    let prod = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("discord_webhook.rs must have a #[cfg(test)] module");

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
