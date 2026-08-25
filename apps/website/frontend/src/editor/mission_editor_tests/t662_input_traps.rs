use crate::editor::arsenal::class_r_scrub::live_code;

/// The keydown region, comment- and dead-code-stripped. The file's first `#[cfg(test)]` is the
/// `clear_for_test` helper near the top, so `live_code` on the whole file would cut everything
/// below it (see the t425/t427 pins); hand it the region from the editor page onward, at a
/// brace-0 boundary so the slice stays balanced. T-934.13 moved the pointer/wheel/contextmenu/
/// dblclick closures to `canvas/gestures.rs`, so that file is appended (scrubbed separately) —
/// the pan-button and contextmenu pins below read those bodies.
fn editor_live() -> String {
    // Full signature (with `()`), so the other test's bare `"pub fn MissionEditorPage"` literal
    // is not a second match. Split so this anchor is not itself a duplicate occurrence.
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src
}

/// (2) Backspace hides chrome and does NOT delete; Delete still deletes. The two keys are split
/// arms — the old combined Delete-or-Backspace alias must be gone (its literal is reassembled
/// below at runtime so this comment is not itself a match).
#[test]
fn backspace_hides_chrome_and_does_not_delete() {
    // String-literal arms: pinned on the RAW file (live_code blanks string literals).
    let raw = include_str!("../mission_editor.rs");
    // Split the needle so the literal below is not itself a second occurrence in this file.
    let combined = format!("{}{}", "\"Delete\" | ", "\"Backspace\"");
    assert!(
        !raw.contains(combined.as_str()),
        "T-662: Backspace must no longer be aliased to Delete — the combined match arm is the bug"
    );
    // T-780 widened the arm's BODY (a map-selected connection line is deleted through
    // `editor_ops::delete_connection`, the panel's own verb; everything else still falls to
    // `delete_selection`), so this pin no longer matches the one-expression form it was written
    // against. The CLAIM is unchanged and is what is asserted: `Delete` alone is still its own
    // arm, and it still removes the selection. Scoped to the window between the two arms, which
    // is the same unambiguous window the Backspace behaviour check below uses.
    let del_at = raw
        .find("\"Delete\" if !modk =>")
        .expect("Delete alone must still be its own match arm");
    let bs_arm = raw
        .find("\"Backspace\" if !modk =>")
        .expect("Backspace must be its own match arm");
    assert!(
        del_at < bs_arm,
        "the Delete arm must still precede the Backspace arm"
    );
    assert!(
        raw[del_at..bs_arm].contains("editor_ops::delete_selection()"),
        "Delete alone must still remove the selection"
    );

    // Behaviour of the Backspace arm: it toggles chrome_hidden and does NOT call delete. The
    // guard is a string literal (blanked by live_code), so this is scoped on the raw file — the
    // only text between the "Backspace" arm and the catch-all `_ =>` is the T-662 note, which
    // does not contain the token `delete_selection`. The keydown region is the only place these
    // arms appear, so the window is unambiguous.
    let bs_at = raw
        .find("\"Backspace\" if !modk =>")
        .expect("Backspace arm present");
    let after = &raw[bs_at
        ..raw[bs_at..]
            .find("_ =>")
            .map(|i| bs_at + i)
            .unwrap_or(raw.len())];
    assert!(
        after.contains("chrome_hidden.set("),
        "the Backspace arm must toggle chrome_hidden (hide the interface)"
    );
    assert!(
        !after.contains("delete_selection"),
        "the Backspace arm must NOT delete the selection"
    );
}

/// (2 cont.) `chrome_hidden` is a real signal that gates the chrome mounts (strip + both docks +
/// the two T-636 bottom mounts: the mode toolbar AND the full-width status bar + the T-667
/// map-pane grid-reference overlay). Declared once, and each mount is wrapped in a
/// `!chrome_hidden.get()` gate.
///
/// T-636 [wave101 N-5]: the split turned the single `BottomToolbelt` gate into TWO (ModeToolbar
/// + StatusBar), so the deliberate count moved 4 → 5. T-667 [wave 106]: the map-pane grid
/// references (`MapGridRefs`) are the same kind of map furniture as the scale bar and must hide
/// with the rest of the chrome on Backspace, so the deliberate count moves 5 → 6. T-648 [wave
/// 110]: the snap-grid step readout (`SnapReadout`) is status-bar furniture like the scale bar /
/// grid refs and must hide with the chrome too, so the deliberate count moves 6 → 7. Pinned as an
/// exact count so a mount can never silently escape the hide-chrome gate (or a stray gate creep
/// in unnoticed) — a legitimate new gated mount UPDATES this number on purpose (it is never
/// bumped to make a red test pass without a matching, intended mount).
#[test]
fn chrome_hidden_signal_gates_the_five_mounts() {
    let ed = editor_live();
    assert!(
        ed.contains("let chrome_hidden = RwSignal::new(false)"),
        "chrome_hidden must be a real RwSignal declared on the page"
    );
    // Each chrome mount must sit behind a chrome_hidden gate. Count the gate wrappers: strip,
    // DockLeft, DockRight, ModeToolbar, StatusBar, MapGridRefs, SnapReadout = 7 (T-636 split +
    // T-667 refs + T-648 snap readout).
    let gates = ed.matches("(!chrome_hidden.get()).then(").count();
    assert_eq!(
        gates, 7,
        "exactly seven chrome mounts (strip + both docks + mode toolbar + status bar + grid refs \
         + snap readout) must be gated on chrome_hidden; found {gates} gate(s)"
    );
    // The docked chrome components must appear inside the gated region (sanity: we did not gate
    // empty divs). BottomToolbelt is retired as a mount — the readouts live in StatusBar and the
    // tools in ModeToolbar, both gated; the T-667 grid refs + T-648 snap readout are gated too.
    assert!(
        ed.contains("TopCommandStrip")
            && ed.contains("DockLeft")
            && ed.contains("DockRight")
            && ed.contains("ModeToolbar")
            && ed.contains("StatusBar")
            && ed.contains("MapGridRefs")
            && ed.contains("SnapReadout"),
        "the gated mounts must still be the real chrome components (incl. the two T-636 halves \
         and the T-667 grid-reference overlay)"
    );
    // Modals must NOT be swept into the hide: a Settings/Attributes dialog survives the toggle.
    // The Attributes modal mount is outside every gate.
    assert!(
        ed.contains("AttributesModal"),
        "the Attributes modal mount must still exist (ungated)"
    );
}

/// (1) RMB no longer pans. The pan branch fires on the middle button only; the old
/// `|| ev.button() == 2` right-button branch that ate the click is gone.
#[test]
fn rmb_no_longer_pans() {
    let ed = editor_live();
    assert!(
        ed.contains("if ev.button() == 1 {"),
        "the pan gesture must start on the middle button (1) only"
    );
    // The whole point: RMB (2) must not be OR-ed into the pan guard anymore.
    assert!(
        !ed.contains("ev.button() == 1 || ev.button() == 2"),
        "T-662: RMB (button 2) must no longer be OR-ed into the pan branch — that OR was the trap"
    );
}

/// (1 cont.) The contextmenu handler keeps `prevent_default` (stop the BROWSER menu) but must
/// NOT `stop_propagation` — the event has to stay reachable for T-664 to attach its menu.
#[test]
fn contextmenu_is_unsuppressed_but_stops_the_browser_menu() {
    let ed = editor_live();
    // Isolate the oncontextmenu closure body.
    let cm_at = ed
        .find("let oncontextmenu =")
        .expect("oncontextmenu closure present");
    // Window up to the next `let on` binding (onpointerleave follows it).
    let rest = &ed[cm_at..];
    let end = rest[3..]
        .find("let on")
        .map(|i| i + 3)
        .unwrap_or(rest.len());
    let body = &rest[..end];
    assert!(
        body.contains("ev.prevent_default()"),
        "contextmenu must still prevent_default to stop the browser's native menu"
    );
    assert!(
        !body.contains("stop_propagation"),
        "contextmenu must NOT stop_propagation — RMB must stay a clean event T-664 can hook"
    );
}
