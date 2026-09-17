//! The copy button may not report a copy it never confirmed.

/// The shipped defect was three lines: a `write_text` call with its promise dropped, then an
/// unconditional success toast. `writeText` rejects on an insecure context, an unfocused
/// document and a denied permission, so over plain http the toast said "copied" while the
/// clipboard was untouched.
///
/// This is a **source** pin rather than a behavioural one, deliberately and with its limits
/// stated: the thing under test is a `navigator.clipboard` promise, which does not exist in a
/// native `cargo test` process at all, and granting a headless browser clipboard permission
/// would test the browser rather than the button. What can be pinned without a browser is
/// *which path the button takes* — and since [`crate::v2::apps::editor::shell::document_commands::write_clipboard`]'s
/// await-then-report contract is pinned in turn by
/// `class_r_write_clipboard_toasts_only_on_the_resolve_arm`, the two together say: this button
/// reaches the one helper, and that helper only claims success after the promise resolved.
///
/// It reads through `class_r_scrub::live_code`, which cuts test modules before scanning — a
/// bare source read would match the needles in these very assertions and stay green with the
/// production code deleted.
#[test]
fn class_r_copy_address_routes_through_the_awaited_clipboard_helper() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let src = crate::v2::core::test_support::pins::server_intel_source();
    let production = live_code(&src);
    let body = only_body(&production, "let copy_address = move |_|");

    assert!(
        body.contains("crate::v2::apps::editor::shell::document_commands::write_clipboard("),
        "the Copy button must copy through the one awaited clipboard helper; got:\n{body}"
    );
    // The two halves of the original defect, each forbidden on its own so that re-introducing
    // either — a raw write, or a success toast the panel decides for itself — is RED.
    assert!(
        !body.contains("write_text"),
        "no raw navigator.clipboard.writeText in the panel — its promise is what got dropped; \
         got:\n{body}"
    );
    assert!(
        !body.contains(".clipboard()"),
        "do not reach for navigator.clipboard here; write_clipboard resolves it and refuses \
         readably when it is absent; got:\n{body}"
    );
    assert!(
        !body.contains("success("),
        "the panel must not toast success itself — only write_clipboard's resolve arm may; \
         got:\n{body}"
    );
}
