use super::keymap_census;
use super::{Shortcut, GROUPS, SHORTCUTS};
use crate::v2::core::test_support::class_r_scrub::live_code;
use std::collections::BTreeSet;

/// Every window-level editor keydown listener, as one set of bound codes. One line, because the
/// extractor lives in exactly one place now (T-738).
fn all_bound() -> BTreeSet<String> {
    keymap_census::all_bound_codes()
}

fn documented() -> BTreeSet<String> {
    SHORTCUTS
        .iter()
        .flat_map(|s| s.codes.iter().map(|c| (*c).to_string()))
        .collect()
}

/// The extractor itself must be honest before either coverage assertion means anything: an
/// extractor that returns nothing would make "every binding is documented" vacuously true, and
/// that is exactly the shape of a pin that passes forever while the UI rots. Pin arms that
/// prove each SOURCE was really parsed and a floor on the count.
///
/// T-703 STRENGTHENED this rather than replacing it. It used to prove two files were read;
/// there were never two, there were six, and the four it could not see were the reason a
/// binding could ship undocumented with every pin green. `ArrowUp` is the witness that carries
/// the widening: no `ev.code()` match anywhere binds it, so it can only have come from an
/// `ev.key()` listener.
#[test]
fn the_extractor_actually_reads_both_keydowns() {
    let bound = all_bound();
    assert!(
        bound.contains("Backspace"),
        "extractor must see mission_editor's arms (found {bound:?})"
    );
    assert!(
        bound.contains("KeyZ"),
        "extractor must see the undo/redo listener's arms (found {bound:?})"
    );
    assert!(
        bound.contains("ArrowUp") && bound.contains("Enter"),
        "T-738: the extractor must see the `ev.key()` listeners too — `ArrowUp` and `Enter` are \
         bound by `context_menu`'s window keydown and by no `ev.code()` match at all, so their \
         absence means the widening has been undone (found {bound:?})"
    );
    assert!(
        bound.contains("Escape"),
        "the shared Escape channel must be censused (found {bound:?})"
    );
    assert!(
        bound.len() >= 20,
        "the editor binds twenty-odd codes; an extractor finding {} has broken, and a broken \
         extractor makes the coverage pins pass vacuously",
        bound.len()
    );
    // A body literal must never be read as a binding. `center_on_selection` is called from the
    // Space arm's body; no arm-head literal may look like a call fragment.
    assert!(
        bound.iter().all(|c| !c.contains('(')),
        "arm-head extraction picked up something that is not a key code: {bound:?}"
    );
}

/// T-738 — the wave-112 MINOR-4 sites (and the two T-774 added) must each appear in the census
/// as an `ev.key()` Escape claim. `ArrowUp`/`Enter` already witness that *some* `ev.key()`
/// listener is read; this pin fails if a *known* Escape site is dropped from the scrape while
/// Escape still arrives from the editor keydown's `ev.code()` arm alone — the exact false-green
/// shape the ticket names.
#[test]
fn known_escape_ev_key_sites_are_censused() {
    let required: &[(&str, &str)] = &[
        // wave-112 MINOR-4 (the three overlays moved file at T-934.11)
        ("overlays.rs", "asset picker / comment / connections"),
        ("attributes.rs", "Attributes modal"),
        ("top_strip_view.rs", "menus / Save / Controls Hint"),
        // already on the surface when T-703 widened; still a drop-from-scrape trap
        ("context_menu.rs", "context menu"),
        ("mission_dialog.rs", "mission settings dialog"),
        ("preferences_dialog.rs", "editor preferences dialog"),
        ("all_settings_dialog.rs", "all settings dialog"),
        // T-774 — the two the eleven-listener census still missed
        ("faction_manager.rs", "Faction Manager"),
        ("dialog_lifecycle.rs", "ORBAT Manager"),
    ];
    let all = keymap_census::all_bindings();
    for (file, what) in required {
        assert!(
            all.iter()
                .any(|b| b.file == *file && b.code == "Escape" && b.via == "ev.key()"),
            "T-738: Escape via `ev.key()` in {file} ({what}) is invisible to the census — that                  is the wave-112 MINOR-4 false-green. Re-add the file to `editor_surface` / teach                  the extractor the idiom; do not document Escape from the `ev.code()` arm alone."
        );
    }
}

/// T-738 — the Escape help row must document the SHARED channel, not only measurement dismissal.
/// The Controls Hint's own close button advertises Esc; a row that names only the ruler lies
/// to the operator standing on that button.
#[test]
fn escape_help_documents_the_shared_channel() {
    let row = SHORTCUTS
        .iter()
        .find(|s| s.codes.contains(&"Escape"))
        .expect("SHORTCUTS must document Escape");
    for needle in [
        "measurement",
        "Save",
        "Attributes",
        "asset picker",
        "settings",
        "context menu",
        "Faction",
        "ORBAT",
        "this card",
    ] {
        assert!(
            row.action.contains(needle),
            "T-738: Escape help action must document the shared channel (missing `{needle}` in                  `{}`); naming only measurement dismissal is the live defect wave-112 MINOR-4 saw",
            row.action
        );
    }
}

/// THE TICKET, as a test: a binding with no help entry is RED. This is the pin that fires when
/// a future slice adds a keydown arm and forgets the operator.
#[test]
fn every_binding_has_a_help_entry() {
    let bound = all_bound();
    let documented = documented();
    let undocumented: Vec<&String> = bound.difference(&documented).collect();
    assert!(
        undocumented.is_empty(),
        "T-692: these editor keydown arms are bound but documented NOWHERE in the UI: \
         {undocumented:?}. Add a `Shortcut` row to `SHORTCUTS` naming each code — that is the \
         whole point of this ticket ('TBD binds keyboard shortcuts and documents none of \
         them'); a new binding must not re-open the defect."
    );
}

/// The other direction: the help surface must not promise a shortcut the editor does not bind.
/// A phantom row is the same lie as a missing one, told the other way round.
#[test]
fn no_help_entry_invents_a_binding() {
    let bound = all_bound();
    let documented = documented();
    let phantom: Vec<&String> = documented.difference(&bound).collect();
    assert!(
        phantom.is_empty(),
        "T-692: `SHORTCUTS` documents {phantom:?}, which no editor keydown arm binds. Either \
         the binding was removed (drop the row) or the code was mistyped."
    );
}

/// T-740 — the undo/redo keydown accepts ctrl OR meta on KeyY (same as undo on KeyZ). A help
/// chord that documents bare `Ctrl + Y` alone lies to Mac operators and is RED.
#[test]
fn redo_chord_documents_cmd_for_key_y() {
    let row = SHORTCUTS
        .iter()
        .find(|s| s.codes.contains(&"KeyY") && s.action == "Redo")
        .expect("SHORTCUTS must document KeyY redo");
    assert!(
        row.chord.contains("Ctrl/Cmd + Y"),
        "T-740: KeyY redo chord must document Cmd (got `{}`); the undo/redo keydown uses \
         ctrl_key() || meta_key() — bare `Ctrl + Y` alone is a lie on Mac",
        row.chord
    );
}

/// Every row is renderable and files under a real heading — a row whose `group` is a typo would
/// silently render nowhere, which is a documented-but-invisible shortcut.
#[test]
fn every_row_renders_under_a_real_group() {
    for Shortcut {
        codes,
        chord,
        action,
        group,
    } in SHORTCUTS
    {
        assert!(!codes.is_empty(), "row `{chord}` names no key code");
        assert!(
            !chord.is_empty() && !action.is_empty(),
            "row {codes:?} is blank"
        );
        assert!(
            GROUPS.contains(group),
            "row `{chord}` files under `{group}`, which is not a rendered heading — it would \
             be invisible"
        );
    }
}

/// The `chrome_hidden` gate, held in place structurally: the overlay is mounted by
/// `eden_top_strip`, and `mission_editor` mounts that strip INSIDE a `(!chrome_hidden.get())`
/// gate with nothing closing the gated block in between. So Backspace takes the hint card with
/// the rest of the chrome, and no second gate can drift away from the first.
#[test]
fn overlay_hides_with_the_rest_of_the_chrome() {
    let strip = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/top_strip/view/overlays.rs"
    )));
    assert!(
        strip.contains("ControlsHint"),
        "the Controls Hint must be mounted from the top strip (that is what puts it behind the \
         chrome_hidden gate)"
    );
    // `live_code` on the WHOLE editor file would blank the mount: the Eden view is inside a
    // `#[cfg(target_arch = "wasm32")]` item, which the scrubber (correctly) treats as dead on
    // the native shell. Hand it the region from the page fn onward, at a brace-0 boundary —
    // the same `editor_live()` manoeuvre the T-662 pins use for the same reason.
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/mission_editor.rs"
    ));
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let ed = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    let mount = ed
        .find("TopCommandStrip")
        .expect("mission_editor mounts the top strip");
    let gate = ed[..mount]
        .rfind("(!chrome_hidden.get()).then(")
        .expect("the strip mount must sit behind the chrome_hidden gate");
    assert!(
        !ed[gate..mount].contains("})}"),
        "the chrome_hidden gate must still be OPEN at the TopCommandStrip mount — otherwise \
         the strip (and the Controls Hint inside it) survives Backspace"
    );
}
