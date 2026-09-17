//! Controls hint menu tests for the top command strip.

/// T-692 — the help surface's TOP-STRIP half: a Help menu in the bar (MENU-BAR-008 / MENU-HELP-001)
/// reaching the one Controls Hint. T-797 F-15 removed the second door (the View-menu MENU-VIEW-017
/// toggle) so the reference now has a single home; the `the_hint_has_exactly_one_home_and_it_is_help`
/// pin below enforces that.
///
/// The list's contents are pinned against the real keydown arms in `eden_help`; what this module
/// pins is that the surface is REACHABLE — a shortcut table nothing opens documents nothing. The
/// menus are a `const` table, so these read it directly rather than scraping source.
use super::{MenuAction, MENUS};

/// Which menu labels carry a row that opens the Controls Hint.
fn menus_reaching_the_hint() -> Vec<&'static str> {
    MENUS
        .iter()
        .filter(|(_, items)| {
            items
                .iter()
                .any(|it| matches!(it.action, Some(MenuAction::ControlsHint)))
        })
        .map(|(name, _)| *name)
        .collect()
}

/// MENU-BAR-008 / MENU-HELP-001 — there is a Help menu, and it is not a stub: its rows are all
/// live commands. A Help menu whose only row is disabled would be the same silence, dressed up.
#[test]
fn the_bar_has_a_live_help_menu() {
    let help = MENUS
        .iter()
        .find(|(name, _)| *name == "Help")
        .expect("T-692: the top strip menu bar must carry a Help menu (MENU-BAR-008)");
    assert!(
        !help.1.is_empty() && help.1.iter().all(|it| it.action.is_some()),
        "T-692: every Help row must be a live command — a disabled-only Help menu documents \
         nothing (MENU-HELP-001)"
    );
}

/// T-797 F-15 — the Controls Hint has ONE home, and it is Help > Keyboard Shortcuts. It used to
/// be reachable from BOTH Help and the View menu (the old MENU-VIEW-017 toggle), which the
/// operator pass called a duplicate: two doors to one overlay is the kind of "where do I find
/// the shortcuts" ambiguity a single home removes. The View menu itself is gone (F-14 + F-15
/// emptied it), so the pin now asserts Help is the SOLE menu reaching the hint — the inverse of
/// the "both" it used to require, and the acceptance's "exactly one Controls Hint entry".
#[test]
fn the_hint_has_exactly_one_home_and_it_is_help() {
    let reaching = menus_reaching_the_hint();
    assert_eq!(
        reaching,
        vec!["Help"],
        "T-797 F-15: the Controls Hint must be reachable from Help ALONE (exactly one entry \
         across all menus); the View-menu duplicate was dropped (found {reaching:?})"
    );
}

/// The action is a TOGGLE reflected in the T-668 checkmark gutter, and the overlay is mounted
/// from this file — specifically inside `TopCommandStrip`'s body (which is what puts it behind
/// `mission_editor`'s `chrome_hidden` gate — the structural half is pinned in `eden_help`).
/// Wave-115 NIT-1 / T-755 / wave-134 F3: presence alone is hollow; the pin must defend POSITION
/// in the gated subtree (between STRIP_ROWS open and its matching close), not merely that the
/// mount string exists somewhere after the open tag.
#[test]
fn the_toggle_is_checked_in_the_gutter_and_mounted_here() {
    let code =
        crate::v2::core::test_support::class_r_scrub::live_code(include_str!("../../top_strip.rs"));
    let body =
        crate::v2::core::test_support::class_r_scrub::only_body(&code, "pub fn TopCommandStrip(");
    let mount = format!("{} open=hint_open", "ControlsHint");
    let mount_at = body.find(&mount).expect(
        "T-692/T-755: the Controls Hint must be mounted inside TopCommandStrip's body — that              is the chrome_hidden-gated subtree",
    );
    // Inside the strip shell — between STRIP_ROWS open and its matching `</div>`, not a sibling
    // above OR after the close (wave-115 NIT-1 / T-755; wave-134 F3 closes the after-close gap).
    let strip_at = body
        .find("STRIP_ROWS")
        .expect("T-692/T-755: TopCommandStrip must still open with STRIP_ROWS");
    let strip_div = body[..strip_at]
        .rfind("<div")
        .expect("T-692/wave-134: STRIP_ROWS must be a <div class=…> open tag");
    let strip_close = {
        // Div-balance from the STRIP_ROWS open to its matching close.
        let bytes = body.as_bytes();
        let mut i = strip_div;
        let mut depth = 0i32;
        let close = loop {
            if i >= body.len() {
                panic!("T-692/wave-134: STRIP_ROWS <div> never closed");
            }
            if body[i..].starts_with("<div") {
                let gt = body[i..].find('>').expect("unclosed <div");
                let self_closing = bytes.get(i + gt - 1) == Some(&b'/');
                if !self_closing {
                    depth += 1;
                }
                i += gt + 1;
            } else if body[i..].starts_with("</div>") {
                depth -= 1;
                if depth == 0 {
                    break i;
                }
                i += 6;
            } else {
                i += 1;
            }
        };
        close
    };
    assert!(
        mount_at > strip_at && mount_at < strip_close,
        "T-692/T-755/wave-134: ControlsHint must sit inside the STRIP_ROWS subtree (between open              and matching close), not beside/above it or after the close"
    );
    // The gutter glyph is reactive on the open latch, so both menus agree on the state.
    assert!(
        body.contains("hint_open.get()"),
        "the checkmark gutter must read the LIVE open state, not a constant"
    );
}
