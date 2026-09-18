use super::*;

#[test]
fn snippet_from_caps_explicit_and_derives_from_body() {
    let long = "x".repeat(250);
    let capped = snippet_from(&long, "ignored");
    assert_eq!(capped.chars().count(), 200);
    assert!(capped.ends_with('…'));

    let derived = snippet_from("", "a < b & c");
    assert_eq!(derived, "a < b & c");
    assert!(!derived.contains("&lt;"));
}

/// Class-R: body-only PATCH must rederive `snippet` on write. Without this, the list view keeps
/// the OLD teaser under a NEW body. Explicit `snippet` in the same PATCH still wins (no
/// auto-overwrite of a caller-supplied teaser).
///
/// RED perturbations:
/// - drop the `input.snippet.is_none()` + `snippet_from("", b)` arm → FAIL
/// - move rederive to a read-time SELECT only → FAIL (handler must write the column)
#[test]
fn body_only_patch_rederives_snippet_on_write() {
    const SRC: &str = include_str!("../announcements_admin.rs");
    let prod = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("announcements_admin.rs must have a #[cfg(test)] module");

    let start = prod
        .find("pub async fn update_announcement")
        .expect("update_announcement must exist");
    let after = &prod[start..];
    let end = after[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let handler = &after[..end];

    assert!(
        handler.contains("if input.snippet.is_none()"),
        "body-only PATCH must gate rederive on snippet absence (perturbation: always skip)"
    );
    assert!(
        handler.contains("snippet_from(\"\", b)"),
        "body-only PATCH must call snippet_from(\"\", b) to derive on write"
    );
    // Assembled so a bait comment cannot satisfy — the bind must sit next to the body arm.
    let body_arm = handler
        .find("if let Some(b) = &input.body")
        .expect("PATCH must bind body via if let Some(b)");
    let body_win = &handler[body_arm..handler.len().min(body_arm + 420)];
    assert!(
        body_win.contains("snippet_from"),
        "snippet rederive must live inside the body-present arm (not a distant comment)"
    );
    assert!(
        body_win.contains("snippet = "),
        "rederive must SET snippet on write (list must not recompute per row)"
    );
}

/// Class-R: the CMS list must be AdminUser-gated and filter drafts+published on both the count
/// and list SQL (not published-only; not a bait comment).
///
/// RED perturbations:
/// - B1: leave `status IN ('draft', 'published')` in a comment + published-only SQL → FAIL
///   (filter must sit in both `query_scalar` and `query_as` windows; count == 2).
/// - M1: drop `_a: AdminUser` from `list_cms_announcements` → FAIL (handler-slice pin).
#[test]
fn list_cms_announcements_is_drafts_plus_published_not_public_feed() {
    const SRC: &str = include_str!("../announcements_admin.rs");
    let prod = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("announcements_admin.rs must have a #[cfg(test)] module");

    let start = prod
        .find("pub async fn list_cms_announcements")
        .expect("CMS GET list handler must exist (perturbation: remove list_cms_announcements)");
    let after = &prod[start..];
    // Next sibling `pub async fn` ends the handler (create_announcement follows today).
    let end = after[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let handler = &after[..end];

    // M1 — AdminUser on THIS handler (sibling extractors elsewhere must not satisfy).
    let admin_pin = format!("{}{}", "_a: ", "AdminUser");
    assert!(
        handler.contains(&admin_pin),
        "list_cms_announcements must take `{admin_pin}` (perturbation: remove AdminUser)"
    );

    // B1 — drafts+published filter on both count (`query_scalar`) and list (`query_as`).
    // Assembled so a free-floating bait comment / this test's source cannot false-green.
    let filter = format!("{}{}", "status IN ('draft', ", "'published')");
    assert_eq!(
        handler.matches(&filter).count(),
        2,
        "count + list SQL must each use `{filter}` (bait comment alone / published-only FAIL)"
    );

    let qs = handler
        .find("query_scalar")
        .expect("list_cms_announcements must use query_scalar for total");
    let qs_win = &handler[qs..handler.len().min(qs + 280)];
    assert!(
        qs_win.contains(&filter),
        "count query_scalar window must contain `{filter}`"
    );

    let qa = handler
        .find("query_as")
        .expect("list_cms_announcements must use query_as for rows");
    let qa_win = &handler[qa..handler.len().min(qa + 520)];
    assert!(
        qa_win.contains(&filter),
        "list query_as window must contain `{filter}`"
    );

    // The registration lives in the community_content route table, which `core::http_router`
    // merges under `/api/v1` — pin both methods there so a create-only registration cannot
    // Class-R green behind a listing handler that still compiles.
    const ROUTES: &str = include_str!("../../routes.rs");
    assert!(
        ROUTES.contains("get(handlers::announcements_admin::list_cms_announcements)"),
        "community_content/routes.rs must register GET /cms/announcements"
    );
    assert!(
        ROUTES.contains(".post(handlers::announcements_admin::create_announcement)"),
        "community_content/routes.rs must register POST on that same /cms/announcements route"
    );
}
