//! Validation chip tests for the top command strip.

/// T-798 — the validation error chip in the top strip (F-11 / F-35 / F-36 + operator decision 3). The
/// floating card is retired; the chip reads the headless eval loop's sink, drops the findings list on
/// click, hides on Backspace by living inside the chrome_hidden-gated strip, and wears an AA-contrast
/// red. Source-scrub pins (the mechanical lane); the live-DOM acceptance (chip '1 error' at load,
/// count unchanged on a marker, Backspace clean-screenshot diff, contrast calc) is the playtest lane.
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// THE CHIP EXISTS, IN THE STRIP, READING THE SEAM. The count comes from the headless eval loop
/// via `validation_panel::chip_findings`, and the drop is `validation_panel::findings_dropdown` —
/// so the strip does not re-implement the findings vocabulary, it renders the one home's output.
#[test]
fn the_chip_reads_the_validation_seam() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    for needle in [
        "validation_open",                     // the chip's own latch
        "validation_panel::chip_findings",     // reads the headless eval loop's sink
        "validation_panel::findings_dropdown", // renders the pinned list + legend
    ] {
        assert!(
            body.contains(needle),
            "T-798: the strip's validation chip must use `{needle}`. Hollow: drop it → the chip \
             stops reflecting validation and the F-11 fix (count at load) is gone."
        );
    }
    // The chip's DOM handle for the live acceptance (the gate reads data-issue-total off it).
    let lit = live_source(super::test_source::top_strip_source());
    let body_lit = only_body(&lit, "pub fn TopCommandStrip(");
    assert!(
        body_lit.contains("data-validation-chip") && body_lit.contains("data-issue-total"),
        "T-798: the chip needs `data-validation-chip` + `data-issue-total` so the scripted \
         acceptance can read the count at load and after a marker place."
    );
}

/// F-36 — CONTRAST. The error count must wear `text-error-alert` (#f87171, ≥4.5:1 on the chrome
/// plate), NOT `text-error` (#ef4444, the app's single 3.9:1 WCAG failure the review measured).
/// `live_source` keeps class strings (the colour is a class literal).
#[test]
fn the_error_count_uses_the_aa_contrast_red() {
    let lit = live_source(super::test_source::top_strip_source());
    let body = only_body(&lit, "pub fn TopCommandStrip(");
    // Scope to the CHIP's accent decision, not the whole strip: the Save dialog's rejected-save
    // list wears its own `text-error` on a different (passing) plate and is out of this finding's
    // scope (the review flagged the COUNT chip alone). The chip's accent is the closure guarded on
    // `has_blocking()` immediately after the `data-validation-chip` marker.
    let chip_at = body
        .find("data-validation-chip")
        .expect("T-798: the validation chip must exist");
    let acc_at = body[chip_at..]
        .find("has_blocking()")
        .map(|o| chip_at + o)
        .expect("T-798: the chip's accent must branch on has_blocking()");
    // The accent region: from has_blocking() through its short if/else colour ladder.
    let mut end = (acc_at + 220).min(body.len());
    while end < body.len() && !body.is_char_boundary(end) {
        end += 1;
    }
    let region = &body[acc_at..end];
    assert!(
        region.contains("text-error-alert"),
        "T-798 (F-36): the chip's blocking-error count must be `text-error-alert` (#f87171, \
         ≥4.5:1). Hollow: swap it to `text-error` → the app's one WCAG failure returns.\n{region}"
    );
    // …and NOT the failing `text-error` (#ef4444, 3.9:1) inside that same accent ladder.
    assert!(
        !region.contains("text-error\""),
        "T-798 (F-36): the failing `text-error` (#ef4444, 3.9:1) must not paint the chip count; \
         use `text-error-alert`.\n{region}"
    );
}

/// TRANSIENT, NOT A DIALOG. The dropdown rides the strip's existing transient machinery — it
/// joins `close_transients`, the ONE Escape closure, and the click-away scrim — and is
/// deliberately NOT registered in the modal stack (a count popover must not steal Escape from an
/// open dialog). This is the deviation the ticket asked be stated: dropdown = menu-class transient.
#[test]
fn the_dropdown_is_a_transient_not_a_modal_dialog() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    // (1) close_transients clears it (opening a dialog / another popover closes the chip).
    let ct_at = body
        .find("let close_transients =")
        .expect("T-798: close_transients closure must exist");
    let ct_region = &body[ct_at..ct_at + 200.min(body.len() - ct_at)];
    assert!(
        ct_region.contains("validation_open.set(false)"),
        "T-798: close_transients must clear validation_open so opening a dialog closes the chip \
         dropdown (one popover up at a time)."
    );
    // (2) the ONE Escape closure closes it — one surface per press.
    let esc_at = body
        .find("escape_consumed()")
        .expect("T-798: the strip's Esc arm must exist");
    let esc_region = &body[esc_at..];
    assert!(
        esc_region.contains("validation_open.get_untracked()")
            && esc_region.contains("validation_open.set(false)"),
        "T-798: the validation dropdown must close on Esc through the strip's ONE closure — not a \
         new window listener (the Esc pile-up the strip already avoids)."
    );
    // (3) NOT a modal_stack Dialog. The chip's latch must never be registered as a modal — it is
    // a menu-class transient. (register_transient_closer is the T-814 strip-owned closer and is
    // fine; `register(` / a Dialog wrapper around validation_open is what is forbidden.)
    let lit = live_source(super::test_source::top_strip_source());
    let body_lit = only_body(&lit, "pub fn TopCommandStrip(");
    assert!(
        !body_lit.contains("<Dialog open=validation_open")
            && !body_lit.contains("modal_stack::register(move || validation_open"),
        "T-798 deviation: the dropdown is a transient, not a Dialog — validation_open must not be \
         registered in the modal stack (it must not own Escape over a real dialog)."
    );
}

/// ANCHORED DROPDOWN, NO PORTAL. The drop uses `MENU_PANEL` (absolute, anchored to the chip), like
/// the Export menu — NOT `position:fixed`, which the strip's `backdrop-blur-xl` containing block
/// would mis-centre (the Save-dialog portal trap). No portal needed, no rect-smoke regression.
#[test]
fn the_dropdown_is_anchored_via_menu_panel() {
    let lit = live_source(super::test_source::top_strip_source());
    let body = only_body(&lit, "pub fn TopCommandStrip(");
    // The validation dropdown's surface reuses MENU_PANEL (the export menu's anchored recipe).
    let drop_at = body
        .find("validation_panel::findings_dropdown")
        .expect("T-798: the chip must render findings_dropdown");
    // Look just before the dropdown body for its container class — MENU_PANEL, anchored right.
    let start = drop_at.saturating_sub(200);
    let region = &body[start..drop_at];
    assert!(
        region.contains("MENU_PANEL"),
        "T-798: the chip's dropdown must reuse MENU_PANEL (absolute/anchored), the export-menu \
         idiom — NOT a fixed-positioned panel the strip's backdrop-filter would mis-centre."
    );
}
