//! The clipboard write may only report a copy the browser confirmed.
//!
//! A source pin: the thing under test is a `navigator.clipboard` promise, which does not exist in
//! a native `cargo test` process, and granting a headless browser clipboard permission would test
//! the browser rather than the helper. It reads through `class_r_scrub::live_code`, which cuts
//! test modules before scanning, so the needles in these assertions cannot satisfy it.

#[test]
fn class_r_write_clipboard_toasts_only_on_the_resolve_arm() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/core/utils/clipboard.rs"
    ));
    let production = live_code(SRC);
    let body = only_body(
        &production,
        "pub fn write_clipboard(text: String, ok_message: String, toasts: Toasts)",
    );

    assert!(
        body.contains("JsFuture::from(promise).await"),
        "the writeText promise must be AWAITED, never dropped; got:\n{body}"
    );
    assert!(
        body.contains("Ok(_) => toasts.success(ok_message)"),
        "success may only be reported on the resolve arm of the awaited promise; got:\n{body}"
    );
    assert!(
        body.contains("Err(e) => toasts.error("),
        "a rejected clipboard write must reach the operator, not the bin; got:\n{body}"
    );
    // `let _ = <anything>.write_text` is the fire-and-forget shape this ticket removed from the
    // repo. It must not come back in the helper every caller now trusts.
    assert!(
        !body.contains("let _ ="),
        "no discarded result inside the clipboard helper; got:\n{body}"
    );
}
