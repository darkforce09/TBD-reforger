/// Strip `//` and `/* */` so comment-only needles cannot satisfy live-call pins.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '*' && matches!(chars.peek(), Some('/')) {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// T-286 Class-R — Delete must open an Aegis confirm, not fire api_delete on the first click.
#[test]
fn delete_requires_aegis_confirm_dialog() {
    const SRC: &str = include_str!("../../faction_manager.rs");
    assert!(
        SRC.contains("confirm_delete_open"),
        "faction delete must gate on a confirm_delete_open signal"
    );
    assert!(
        SRC.contains("Delete this faction?"),
        "confirm Dialog title must be present"
    );
    assert!(
        SRC.contains("on:click=move |_| confirm_delete_open.set(true)"),
        "Delete button must open the confirm dialog, not call api_delete directly"
    );
    // The Delete toolbar button must not wire `on:click=delete` (that runs the DELETE).
    assert!(
        !SRC.contains("aria-label=\"Delete faction\" on:click=delete"),
        "toolbar Delete must not invoke api_delete without confirmation"
    );
}

/// T-507 / T-514 Class-R — save must trim `doc.name` before `serde_json::to_value(&doc)`.
/// Pre-fix: empty check used `trim()` then posted the untrimmed string; API
/// `validated_side_name` rejects pad (`name != name.trim()`), so `"USA "` 400'd.
/// T-514: strip comments first — a `// doc.name = doc.name.trim()…` bait must not
/// satisfy the pin while the live assign is gone. Bound to production (before
/// `#[cfg(test)]`) so this test's own string literals cannot satisfy the find.
#[test]
fn save_trims_name_before_serialize() {
    const SRC: &str = include_str!("../../faction_manager.rs");
    let save_start = SRC
        .find("let save = move |_|")
        .expect("save handler present");
    let prod_end = SRC.find("#[cfg(test)]").unwrap_or(SRC.len());
    assert!(
        save_start < prod_end,
        "save handler must live in production code before #[cfg(test)]"
    );
    let save = strip_rust_comments(&SRC[save_start..prod_end]);
    let trim_at = save
        .find("doc.name = doc.name.trim().to_string()")
        .expect("save must assign trimmed name onto doc before post/put");
    let ser_at = save
        .find("serde_json::to_value(&doc)")
        .expect("save must serialize &doc");
    assert!(
        trim_at < ser_at,
        "trim assign must precede serde_json::to_value(&doc) (got trim@{trim_at} ser@{ser_at})"
    );
    // Guard against regressing to trim-empty-only without mutating the payload.
    assert!(
        !save.contains("if doc.name.trim().is_empty()")
            || save.contains("doc.name = doc.name.trim().to_string()"),
        "must not check trim-empty then serialize the untrimmed name"
    );
}

/// T-726 — Faction Manager Esc must gate on modal_stack topmost (wave139 F3).
#[test]
fn faction_manager_gates_escape_on_modal_stack() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let code = live_code(include_str!("../../faction_manager.rs"));
    let body = only_body(&code, "pub fn FactionManagerDialog(");
    let reg = ["modal_stack", "::", "register("].concat();
    let top = ["modal_stack", "::", "is_topmost_open(modal_id)"].concat();
    let unreg = ["modal_stack", "::", "unregister(modal_id)"].concat();
    assert!(
        body.contains(&reg),
        "T-726: FactionManagerDialog must register"
    );
    assert!(
        body.contains(&top),
        "T-726: FactionManagerDialog must gate Escape on is_topmost_open"
    );
    assert!(
        body.contains(&unreg),
        "T-726: FactionManagerDialog must unregister"
    );
}
