use crate::editor::arsenal::class_r_scrub::live_code;

/// The page from its component anchor PLUS the T-934.13 gesture file: the ruler wiring spans the
/// page body (tool signals, keydown Esc, overlay mounts) and the pointer/dblclick closures, which
/// moved verbatim to `canvas/gestures.rs`. Both halves are scrubbed separately (`live_code`
/// truncates at the first `#[cfg(test)]`, and each file has its own tail).
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src.push_str(&live_code(include_str!("../canvas/commands.rs")));
    src
}

/// (tool-mode arbitration — how the third mode ENTERS `LeftGesture`) The LMB pointerdown chooses
/// `LG::Ruler` via `should_begin_ruler(tool_mode, button)` instead of `LG::Pending`. This is the
/// entry point AND constraint (c) — `should_begin_ruler` carries the button-0 filter.
#[test]
fn pointerdown_arbitrates_ruler_via_should_begin_ruler() {
    let ed = editor_live();
    assert!(
        ed.contains("should_begin_ruler("),
        "T-642: pointerdown must arbitrate the ruler via ruler_tool::should_begin_ruler(...)"
    );
    assert!(
        ed.contains("LeftGesture::Ruler {") || ed.contains("select_tool::LeftGesture::Ruler"),
        "T-642: the ruler press must open the LG::Ruler gesture (the third LeftGesture mode)"
    );
}

/// (constraint a — NOT the armed-place branch) The ruler commit lives in an `LG::Ruler` arm, and
/// that arm must NOT contain the armed-place `has_pending()` token nor any doc-move commit — it is
/// a separate arm reached only after the `has_pending()` branch has already returned, so it can
/// never route through the T-723 armed-placement pointerup branch.
#[test]
fn ruler_commit_arm_avoids_armed_place_and_doc_writes() {
    let ed = editor_live();
    // Slice the LG::Ruler pointerup arm: from the LAST "LG::Ruler {" (the pointerup commit; the
    // earlier ones are pointerdown/pointermove which have no commit) to the next "LG::" or arm end.
    let arms: Vec<&str> = ed.split("LG::Ruler").skip(1).collect();
    assert!(!arms.is_empty(), "T-642: an LG::Ruler arm must exist");
    // The commit arm is the one that calls `.press(` on the ruler chain.
    let commit: Vec<&str> = arms
        .iter()
        .map(|a| a.split("LG::").next().unwrap_or(a))
        .filter(|a| a.contains(".press("))
        .collect();
    assert_eq!(
        commit.len(),
        1,
        "T-642: exactly one LG::Ruler arm commits a vertex via chain.press( (found {})",
        commit.len()
    );
    let arm = commit[0];
    // Constraint (a): the ruler commit does NOT sit in / call the armed-place branch.
    assert!(
        !arm.contains("has_pending()"),
        "T-642 (a): the ruler commit must NOT route through the has_pending() armed-place branch"
    );
    // Decision 4 + move_commit invariant: the ruler arm never calls a doc-move commit.
    assert!(
        !arm.contains("move_entities_and_vehicles"),
        "T-642: the ruler commit must not call move_entities_and_vehicles (it is not a doc edit)"
    );
}

/// (constraint b — take/clear any pending) The ruler gesture the pointerdown wrote is always
/// consumed: the pointerup/cancel `left.borrow_mut().take()` clears it (there is exactly one
/// take-into-a-`let` at the top of each of those handlers, shared with the Select gestures), and
/// the pointermove `LG::Ruler` arm puts it back rather than dropping it.
#[test]
fn ruler_gesture_is_taken_and_cleared() {
    let ed = editor_live();
    // The shared take idiom the ruler arm relies on.
    assert!(
        ed.contains("left.borrow_mut().take()"),
        "T-642 (b): the pointer handlers must take() the LeftGesture (clearing any LG::Ruler)"
    );
    // The pointermove keeps the ruler pending (a self → self arm), so a move never loses it.
    assert!(
        ed.matches("LG::Ruler").count() >= 3,
        "T-642 (b): LG::Ruler must appear across pointerdown/move/up (written, kept, committed)"
    );
}

/// (constraint d — Esc disarms) The keydown Escape arm dismisses the ruler chain via
/// `ruler...escape()`. This is Decision 3's two-step dismissal entry from the keyboard.
#[test]
fn escape_dismisses_the_ruler() {
    let ed = editor_live();
    assert!(
        ed.contains(".escape()"),
        "T-642 (d): the keydown Escape arm must call chain.escape() to disarm/clear the ruler"
    );
    // The arm reads the ruler and syncs — it is inside the keydown match on `code().as_str()`
    // (the "Escape" string literal itself is blanked by `live_code`, so pin the surviving
    // structure: the keydown dispatch + the escape() call together prove a real Escape arm).
    assert!(
        ed.contains("code().as_str()") && ed.contains("ruler.borrow_mut().escape()"),
        "T-642 (d): the Escape dismissal must be a keydown arm calling ruler.escape()"
    );
}

/// (dismissal — dbl-click ends the chain) The dblclick handler ends the ruler (dedup + end) when
/// the ruler tool is active, and returns before the Attributes/asset-picker pick.
#[test]
fn dblclick_ends_the_ruler_chain() {
    let ed = editor_live();
    assert!(
        ed.contains(".dedup_tail(") && ed.contains(".double_click()"),
        "T-642: the dblclick handler must dedup + end the ruler chain (double_click keeps it placed)"
    );
}

/// (Decision 4 — session-local, tool-switch clear) Switching the tool back to Select clears the
/// placed ruler, and the chain is registered for the overlay + mounted. Also pins that the chain
/// handle is a leaked `RefCell<RulerChain>` (overlay state), never a doc write.
#[test]
fn tool_switch_clears_and_overlay_is_mounted() {
    let ed = editor_live();
    // Tool-switch clear effect (reads tool_mode, clears the chain).
    assert!(
        ed.contains("is_ruler()") && ed.contains("ruler.borrow_mut().clear()"),
        "T-642 (Decision 3): switching away from Ruler must clear the placed chain"
    );
    // The overlay is mounted + the chain registered for it.
    assert!(
        ed.contains("RulerOverlay") && ed.contains("register_ruler_chain("),
        "T-642: RulerOverlay must be mounted and the chain registered for it"
    );
    // The chain is session-local overlay state (a RulerChain in a RefCell), NOT the Y.Doc.
    assert!(
        ed.contains("RulerChain::new()"),
        "T-642 (Decision 4): the ruler is a session-local RulerChain, not doc state"
    );
}
