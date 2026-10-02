//! Wave-118 NIT-1 / T-772 — the ControlsHint close button must keep a comfortable hit box
//! without widening `eden_layout::BTN_ICON` (dense strip/dock rows need `p-0.5`).
//!
//! Hollow-pin discipline:
//! 1. RED under the original defect (`cn(&[BTN_ICON, HOVER_FILL])` alone).
//! 2. RED under a second hollow (comment/string decoy or conflicting `p-0.5`+`p-1.5` via
//!    shared recipe) — pins require the live `HINT_CLOSE_BTN` identifier in the component body
//!    under `live_code` (literals blanked) and exclusive `p-1.5` on the call-site recipe.

use super::HINT_CLOSE_BTN;
use crate::v2::apps::editor::shell::layout::BTN_ICON;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn hint_body_source() -> String {
    let src = live_source(&super::source::production_source());
    only_body(&src, "pub fn ControlsHint(").to_string()
}

fn hint_body_code() -> String {
    let src = live_code(&super::source::production_source());
    only_body(&src, "pub fn ControlsHint(").to_string()
}

/// Original defect: sizing the overlay dismiss from the dense shared recipe alone.
#[test]
fn close_button_uses_call_site_padding_not_shared_recipe_alone() {
    assert!(
        HINT_CLOSE_BTN.contains("p-1.5") && !HINT_CLOSE_BTN.contains("p-0.5"),
        "T-772: HINT_CLOSE_BTN must be comfortable p-1.5 without the dense p-0.5"
    );
    assert!(
        BTN_ICON.contains("p-0.5") && !BTN_ICON.contains("p-1.5"),
        "T-772: do not widen BTN_ICON — dense-row sizing stays p-0.5"
    );
    assert!(
        HINT_CLOSE_BTN.contains("shrink-0")
            && HINT_CLOSE_BTN.contains("rounded")
            && HINT_CLOSE_BTN.contains("text-on-surface")
            && !HINT_CLOSE_BTN.contains("text-on-surface-variant"),
        "T-772: call-site recipe keeps BTN_ICON's bright rest + geometry, padding excepted"
    );

    let body = hint_body_source();
    assert!(
        body.contains("HINT_CLOSE_BTN") && body.contains("HOVER_FILL"),
        "T-772: ControlsHint close must compose HINT_CLOSE_BTN + HOVER_FILL"
    );
    assert!(
        !body.contains("BTN_ICON"),
        "T-772: ControlsHint close must not size from BTN_ICON (wave118 NIT-1 original defect)"
    );
    // `cn` does not twMerge — forbidding a BTN_ICON co-compose also blocks p-0.5+p-1.5
    // conflict. Do not substring-match `p-0.5` on the whole body: shortcut rows use `py-0.5`.
    assert!(
        !body.contains("\"p-0.5\"") && !body.contains(" p-0.5"),
        "T-772: close call site must not also carry dense padding class p-0.5 (cn cannot \
         resolve the conflict with HINT_CLOSE_BTN's p-1.5)"
    );
}

/// Second hollow: a comment / blanked-literal decoy must not green the pin. `live_code` blanks
/// string literals and comments, so only a real identifier use of `HINT_CLOSE_BTN` survives.
#[test]
fn call_site_padding_identifier_is_live_not_hollow() {
    let body = hint_body_code();
    assert!(
        body.contains("HINT_CLOSE_BTN"),
        "T-772: HINT_CLOSE_BTN must appear as a live cn argument in ControlsHint (not a \
         comment or string decoy — live_code blanks those)"
    );
    assert!(
        body.contains("HOVER_FILL"),
        "T-772: close must still compose HOVER_FILL (behaviour preserved)"
    );
    assert!(
        !body.contains("BTN_ICON"),
        "T-772 hollow: BTN_ICON must not return to the ControlsHint close call"
    );
}
