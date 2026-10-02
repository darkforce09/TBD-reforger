//! Layout t636 band readers agree tests.

//! T-636 / T-638 — `TOOLBELT_BAND_PX` (and the three dock/strip insets) are INPUT-HANDLING numbers:
//! they are the chrome band the pointer→world readers inset by, so if the readers disagree a click
//! under the status bar (or a collapsed dock's freed strip) would be mapped to a world coordinate as
//! if the chrome were not there. This file OWNS the numbers; the two chokepoint readers this ticket
//! owns must consume them through the T-638 **accessors** (`eden_layout::dock_left_px()` etc.) rather
//! than a magic literal OR the frozen expanded const, so a collapse — and any future height change —
//! stays consistent by construction. That is exactly what this pins.
//!
//! It lives here (the consts' owner, natively compiled) rather than in `select_tool`, which is
//! `#[cfg(target_arch = "wasm32")]` and so invisible to a native `cargo test`.

use crate::v2::core::test_support::class_r_scrub::live_code;

/// The band has ONE definition here (its expanded value), and both chokepoint readers reference
/// the LIVE inset via the accessor — neither smuggles in a bare `96.0`/`240.0`/`48.0`.
/// `live_code` blanks comments + string literals, so a number mentioned in prose or a class
/// string can never satisfy (or false-fail) a needle; the definitions themselves are checked on
/// raw source, where the `= 96.0` value is real code.
///
/// T-638 accessor-conversion completeness: the two readers (`select_tool::farthest_empty_px`, the
/// pointer→world probe-grid inset; `mission_editor`'s palette-drop `on_canvas` gate) each moved
/// from `eden_chrome::TOOLBELT_BAND_PX` (a frozen const read) onto `eden_layout::*_px()` (the
/// dynamic accessor), and NEITHER may be left on a hardcoded inset literal.
#[test]
fn both_readers_reference_the_single_band_const() {
    // Exactly one definition per inset, in this file (raw — the literals are real code, not
    // prose). The expanded consts survive (the `eden_chrome` shim + `eden_toolbelt` grid-refs
    // read them by name as bare f64), so the band value is still pinned once.
    let layout = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/layout.rs"
    ));
    let name = "TOOLBELT_BAND_PX";
    let def = format!("pub const {name}: f64 = 96.0;");
    assert!(
        layout.contains(&def),
        "eden_layout must DEFINE {name} exactly once as the band's expanded value (96.0)"
    );
    for (n, one) in [
        ("STRIP_TOP_PX", true),
        ("DOCK_LEFT_PX", true),
        ("DOCK_RIGHT_PX", true),
        ("TOOLBELT_BAND_PX", true),
    ] {
        if one {
            assert_eq!(
                layout.matches(&format!("pub const {n}")).count(),
                1,
                "{n} must have exactly one definition — one source of truth"
            );
        }
    }

    // Reader 1: select_tool::farthest_empty_px (the pointer→world probe-grid inset).
    // Reader 2: mission_editor's palette-drop `on_canvas` gate.
    // Both must consume the LIVE inset through the T-638 accessor, by NAME.
    //
    // `live_code` (via `scrub`) cuts from a file's FIRST `#[cfg(test)]` to EOF. select_tool has
    // none, so its whole body scrubs. mission_editor's first `#[cfg(test)]` is a `clear_for_test`
    // helper near the TOP (above the band reader), so scrubbing the whole file would drop the
    // reader — slice from the page fn anchor first (the t662/t635 idiom), then scrub that.
    let band_read = "editor::shell::layout::toolbelt_band_px()";
    let sel = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/input/tools/select_tool.rs"
    )));

    let raw_editor = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/mission_editor.rs"
    ));
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    assert_eq!(
        raw_editor.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    // The palette-drop `on_canvas` gate lives in the pointer-up handler. Scrub it separately
    // so the reader is examined with the page source.
    let mut editor =
        live_code(&raw_editor[raw_editor.find(anchor.as_str()).expect("anchor present")..]);
    editor.push_str(&live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/input/pointer_gestures/pointer_up.rs"
    ))));

    assert!(
        sel.contains(band_read),
        "select_tool must inset by the accessor {band_read}, not a literal or frozen const"
    );
    assert!(
        editor.contains(band_read),
        "mission_editor's on_canvas gate must inset by the accessor {band_read}"
    );

    // T-638 completeness: every one of the four insets is read via its accessor in BOTH readers
    // (a reader that quietly stops insetting by one axis is caught), and NO reader mentions the
    // old frozen `eden_chrome::CONST` read for these four (that would freeze the axis against
    // collapse). eden_chrome + eden_toolbelt legitimately keep the const NAMES — they are not
    // this ticket's owns and read the expanded value as bare f64 — so they are excluded here.
    for acc in [
        "editor::shell::layout::dock_left_px()",
        "editor::shell::layout::dock_right_px()",
        "editor::shell::layout::strip_top_px()",
        "editor::shell::layout::toolbelt_band_px()",
    ] {
        assert!(
            sel.contains(acc),
            "select_tool must read the live inset via {acc}"
        );
        assert!(
            editor.contains(acc),
            "mission_editor on_canvas must read the live inset via {acc}"
        );
    }
    for frozen in [
        "eden_chrome::DOCK_LEFT_PX",
        "eden_chrome::DOCK_RIGHT_PX",
        "eden_chrome::STRIP_TOP_PX",
        "eden_chrome::TOOLBELT_BAND_PX",
    ] {
        assert!(
            !sel.contains(frozen),
            "select_tool must not read the frozen const {frozen} — collapse needs the accessor"
        );
        assert!(
            !editor.contains(frozen),
            "mission_editor must not read the frozen const {frozen} — use the accessor"
        );
    }

    // No reader may hardcode an inset (that would silently diverge from a collapse). The needles
    // are split so this test's own source cannot satisfy them. eden_layout is excluded — it
    // legitimately holds the literals in the definitions above.
    // T-637: the two dock literals collapsed from 256/320 to one equalised 240 (`DOCK_PX`).
    for bare in [
        ["96", ".0"].concat(),
        ["240", ".0"].concat(),
        ["48", ".0"].concat(),
    ] {
        assert!(
            !sel.contains(&bare),
            "T-638: select_tool must not hardcode an inset ({bare}) — it comes from the accessor"
        );
        assert!(
            !editor.contains(&bare),
            "T-638: mission_editor must not hardcode an inset ({bare}) — use the accessor"
        );
    }
}
