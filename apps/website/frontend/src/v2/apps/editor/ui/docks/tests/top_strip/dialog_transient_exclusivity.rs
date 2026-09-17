//! Dialog transient exclusivity tests for the top command strip.

/// T-786 O-5 — opening a dialog closes the strip's popovers/help surfaces (the Controls Hint).
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The `close_transients` helper must actually close all three transient surfaces — the open
/// menu, the export dropdown, and the Controls Hint — or the exclusivity is hollow. Scrubbed
/// `live_code`, so a doc comment or string cannot satisfy the needles.
#[test]
fn close_transients_closes_menu_export_and_hint() {
    let scrubbed = live_code(include_str!("../../top_strip.rs"));
    let body = only_body(&scrubbed, "pub fn TopCommandStrip(");
    // The definition and its three effects.
    let def_at = body
        .find("let close_transients =")
        .expect("T-786: TopCommandStrip must define close_transients");
    let region = &body[def_at..];
    for needle in [
        "open_menu.set(None)",
        "export_open.set(false)",
        "set_hint(false)",
    ] {
        assert!(
            region.contains(needle),
            "T-786 O-5: close_transients must run `{needle}` so a dialog opening clears it"
        );
    }
}

/// The wiring: every path that OPENS a dialog from the strip must first close the Controls Hint
/// — so "Help + Save Version stack simultaneously" (O-5) can no longer happen. The three buttons
/// go through `close_transients()`; the two dialog-opening menu actions call `set_hint(false)`
/// directly (they already close menu/export at the top of `run_action`).
#[test]
fn every_dialog_open_path_closes_the_controls_hint() {
    let scrubbed = live_code(include_str!("../../top_strip.rs"));
    let body = only_body(&scrubbed, "pub fn TopCommandStrip(");
    // Save Version, Mission Settings, and ORBAT Manager buttons each close transients before
    // opening. Match the open call, then require a hint-close within the handler just above it.
    for (label, open_call) in [
        ("Save Version button", "save_open.set(true)"),
        ("ORBAT Manager button", "o.set(true)"),
        ("Mission Settings button", "s.set(true)"),
    ] {
        let at = body
            .find(open_call)
            .unwrap_or_else(|| panic!("T-786: {label} open call `{open_call}` not found"));
        // The handler is small; look back a short window for the transient-close. Walk the
        // start back to a char boundary so a `—` in a nearby comment cannot split a slice.
        let mut window_start = at.saturating_sub(160);
        while window_start > 0 && !body.is_char_boundary(window_start) {
            window_start -= 1;
        }
        let handler = &body[window_start..at];
        assert!(
            handler.contains("close_transients()") || handler.contains("set_hint(false)"),
            "T-786 O-5: {label} must close the Controls Hint before opening its dialog. \
             Handler window was: {handler}"
        );
    }
    // The menu-action arms for Save and Settings each close the hint directly.
    let run_at = body
        .find("let run_action =")
        .expect("run_action must exist");
    let run_region = &body[run_at..];
    for arm in ["MenuAction::Save =>", "MenuAction::Settings =>"] {
        let arm_at = run_region
            .find(arm)
            .unwrap_or_else(|| panic!("T-786: {arm} arm not found in run_action"));
        let mut arm_end = (arm_at + 160).min(run_region.len());
        while arm_end < run_region.len() && !run_region.is_char_boundary(arm_end) {
            arm_end += 1;
        }
        let arm_region = &run_region[arm_at..arm_end];
        assert!(
            arm_region.contains("set_hint(false)"),
            "T-786 O-5: the `{arm}` action must close the Controls Hint when it opens its dialog"
        );
    }
}
