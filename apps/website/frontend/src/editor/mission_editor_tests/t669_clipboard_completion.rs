/// T-703/T-738 — THE keydown arm-list extractor. This module held the third of four copies;
/// it now consumes the one in `eden_help::keymap_census`, which also carries the structured
/// `(code, modifiers)` census that finally makes the Ctrl+V / Ctrl+Shift+V distinction this
/// module's own pins had to hand-check.
use crate::editor::panels::help_modal::keymap_census::keydown_arms;
use std::collections::BTreeSet;

/// Needles assembled so the arm LITERAL never appears verbatim in this test's own source.
fn key(k: &str) -> String {
    format!("\"{k}\"")
}

/// KEY CENSUS: Ctrl/Cmd+X is claimed by THIS editor keydown and by nothing else. The two
/// window-level editor keydowns are this file's and `mission_history`'s (Ctrl+Z/Y); the other
/// one must not also bind X, or both listeners would fire on one keypress and the selection
/// would be cut twice.
#[test]
fn t669_cut_key_census() {
    let this_arms = keydown_arms(include_str!("../canvas/commands.rs"));
    let history_arms = keydown_arms(include_str!("../state/history.rs"));
    let key_x = key("KeyX");
    assert!(
        !history_arms.contains(&key_x),
        "census: mission_history's keydown (Ctrl+Z/Y) must not claim X"
    );
    // Modifier-gated (Ctrl/Cmd), rejecting Alt and Shift — the same guard shape as the Ctrl+C /
    // Ctrl+V arms it sits between, so a BARE `x` stays free.
    assert!(
        this_arms.contains(&format!(
            "{key_x} if modk && !ev.alt_key() && !ev.shift_key() =>"
        )),
        "ACTION-CUT-001: Ctrl/Cmd+X (not bare X, not an Alt combo) must be the cut arm"
    );
    // The neighbours are untouched: this slice ADDED arms, it did not re-key the existing ones.
    for k in ["KeyC", "KeyA"] {
        assert!(
            this_arms.contains(&format!(
                "{} if modk && !ev.alt_key() && !ev.shift_key() =>",
                key(k)
            )),
            "the existing {k} arm must be unchanged by T-669"
        );
    }
}

/// A cut that could not COPY must not DELETE. `copy_selection` returns false on an empty
/// selection or a doc that is not up, and `&&` short-circuits on that false — so the arm can
/// never degrade into a silent destructive Delete. Order is the contract: copy first.
#[test]
fn cut_copies_before_it_deletes_and_short_circuits() {
    let arms = keydown_arms(include_str!("../canvas/commands.rs"));
    let at = arms
        .find(&format!("{} if modk", key("KeyX")))
        .expect("the cut arm exists — censused above");
    let body = &arms[at..];
    let copy = body
        .find("editor_ops::copy_selection()")
        .expect("ACTION-CUT-001: the cut arm must snapshot the selection to the clipboard");
    let del = body
        .find("editor_ops::delete_selection()")
        .expect("ACTION-CUT-001: the cut arm must then remove the selection");
    assert!(
        copy < del,
        "ACTION-CUT-001: copy must run BEFORE delete — a cut that deletes first has already \
         destroyed what it was supposed to put on the clipboard"
    );
    assert!(
        body[copy..del].contains("&&"),
        "ACTION-CUT-001: the two calls must be joined by `&&` (short-circuit), not sequenced — \
         otherwise a failed copy still deletes and the cut is an undocumented Delete"
    );
}

/// `paste_at_cursor`'s anchor is `Option`al, and that option IS paste-at-original: the plain
/// paste arm hands it a point, the Shift arm hands it nothing so every slot keeps its source
/// coordinates. Pin both halves — passing `cx, cy` to the Shift arm by accident is the exact
/// regression that would make this ticket a no-op while still looking bound.
///
/// **T-743 — the no-anchor call must be UNIQUE.** The plain arm used to pass its raw `cx`/`cy`
/// through, which are `None` with the pointer off the map, so the two commands collapsed onto
/// one branch and the shared branch had to compromise (a 20 m nudge) to serve both. The count
/// below is the whole split, expressed as a fact about the shipped listener: exactly ONE arm in
/// the editor keydown may say "no anchor", and it is the one Shift+V takes. Counted over the
/// entire arm list rather than inside a slice, so a re-ordered or renamed arm cannot make it
/// pass by moving the offender out of a window.
#[test]
fn paste_at_original_passes_no_anchor() {
    let arms = keydown_arms(include_str!("../canvas/commands.rs"));
    let key_v = key("KeyV");
    let plain = arms
        .find(&format!(
            "{key_v} if modk && !ev.alt_key() && !ev.shift_key() =>"
        ))
        .expect("the cursor-anchored paste arm must survive this slice");
    let shifted = arms
        .find(&format!(
            "{key_v} if modk && !ev.alt_key() && ev.shift_key() =>"
        ))
        .expect("ACTION-PASTE-ORIG-001: Ctrl/Cmd+Shift+V must be an arm of its own");
    assert!(
        arms[plain..shifted].contains("plain_paste_anchor(cx.zip(cy), view_centre)"),
        "T-743: the plain Ctrl/Cmd+V must resolve its own anchor — cursor, else the view \
         centre — instead of handing an `Option` straight to the paste"
    );
    assert!(
        arms[plain..shifted].contains("editor_ops::paste_at_cursor(Some(ax), Some(ay))"),
        "T-743: the plain Ctrl/Cmd+V must paste with an anchor it has already resolved"
    );
    assert!(
        arms[shifted..].contains("editor_ops::paste_at_cursor(None, None)"),
        "ACTION-PASTE-ORIG-001: the Shift arm must pass NO anchor — that is what makes the \
         paste land on the source position instead of the cursor"
    );
    assert_eq!(
        arms.matches("editor_ops::paste_at_cursor(None, None)")
            .count(),
        1,
        "T-743: exactly one keydown arm may paste with no anchor. A second one means some \
         other chord silently means paste-at-original too, which is the shared-branch defect \
         this ticket removed"
    );
}

/// **T-743 — the off-map plain paste falls back to the view centre, never to "no anchor".**
///
/// The keydown arm itself cannot run off-browser (it reads a live camera), so the DECISION is
/// factored into a pure function and pinned here; the arm's call site is pinned by source
/// above. Perturb `plain_paste_anchor` to return `None` when the cursor is missing — the
/// off-map case fails, and that perturbation is precisely the regression that would hand
/// `paste_at_cursor` a `None` anchor and turn a plain paste into a paste-at-original.
#[test]
fn t743_plain_paste_falls_back_to_the_view_centre() {
    // Cursor on the map wins outright — the fallback must not override a real cursor.
    assert_eq!(
        super::plain_paste_anchor(Some((10.0, 20.0)), Some((999.0, 999.0))),
        Some((10.0, 20.0)),
        "a live map cursor is the anchor; the view centre is only a fallback"
    );
    // Pointer over a chrome panel (the common case) — anchored on the view centre, and NOT
    // silently promoted to paste-at-original.
    assert_eq!(
        super::plain_paste_anchor(None, Some((640.0, 480.0))),
        Some((640.0, 480.0)),
        "an off-map plain paste must still carry an anchor — the middle of what is on screen"
    );
    // No camera at all (engine not booted, or a singular matrix) — there is nothing to anchor
    // on and nothing on screen, so the keypress does not paste.
    assert_eq!(
        super::plain_paste_anchor(None, None),
        None,
        "with no cursor and no camera the plain paste must decline, not invent a coordinate"
    );
}

/// The two Ctrl/Cmd+V arms PARTITION their key rather than overlapping it. Two halves, because
/// either alone would be weak: the source half reads the real guards out of the live arm list
/// (so it cannot drift from the code), and the truth table then evaluates those exact guard
/// shapes over every `(ctrl/meta, alt, shift)` combination — one `KeyboardEvent` carries exactly
/// one `shiftKey`, so no event can satisfy both, and match ORDER between them is irrelevant.
#[test]
fn the_two_paste_arms_are_mutually_exclusive() {
    let arms = keydown_arms(include_str!("../canvas/commands.rs"));
    let key_v = key("KeyV");
    assert_eq!(
        arms.matches(key_v.as_str()).count(),
        2,
        "V must be bound exactly twice (cursor paste + paste-at-original); a third arm would \
         make this proof incomplete"
    );
    let plain = format!("{key_v} if modk && !ev.alt_key() && !ev.shift_key() =>");
    let shifted = format!("{key_v} if modk && !ev.alt_key() && ev.shift_key() =>");
    assert!(
        arms.contains(&plain) && arms.contains(&shifted),
        "the two V arms must differ ONLY in the polarity of the shift guard — anything else \
         and the exclusivity argument below is about code that is not there"
    );
    // The guards above, evaluated. `modk` is `ctrl || meta`, so the three inputs are exhaustive.
    let plain_guard = |modk: bool, alt: bool, shift: bool| modk && !alt && !shift;
    let shifted_guard = |modk: bool, alt: bool, shift: bool| modk && !alt && shift;
    for modk in [false, true] {
        for alt in [false, true] {
            for shift in [false, true] {
                assert!(
                    !(plain_guard(modk, alt, shift) && shifted_guard(modk, alt, shift)),
                    "the V arms both match at modk={modk} alt={alt} shift={shift} — the \
                     second would be dead code and the binding ambiguous"
                );
                // Together they cover Ctrl/Cmd+V without Alt, and nothing else: an Alt or a
                // bare V still falls through to the arms below and then to `_ => false`.
                assert_eq!(
                    plain_guard(modk, alt, shift) || shifted_guard(modk, alt, shift),
                    modk && !alt,
                    "the V pair must claim exactly Ctrl/Cmd+V (Alt-free) — no more, no less"
                );
            }
        }
    }
}

/// `eden_help`'s coverage pins compare CODE SETS, and paste-at-original re-uses `KeyV`. So a
/// missing help row for Ctrl/Cmd+Shift+V would leave those pins GREEN while the operator has no
/// way to discover the binding — the exact defect T-692 exists to prevent, slipping through the
/// one hole its set comparison cannot see. Pin the two CHORDS instead.
#[test]
fn both_new_chords_are_documented_in_the_help_table() {
    // Raw source: the chords ARE string literals, so a scrub that blanks literals would blank
    // the thing under test.
    let help = include_str!("../panels/help_modal.rs");
    for chord in ["Ctrl/Cmd + X", "Ctrl/Cmd + Shift + V"] {
        assert!(
            help.contains(chord),
            "T-669: `{chord}` is bound by the editor keydown but has no row in \
             `eden_help::SHORTCUTS` — the help surface must not go stale the first time a \
             ticket adds a chord on an already-documented key code"
        );
    }
}

/// The help module's opening sentence counts the bindings, and a hand-typed count goes stale the
/// moment a slice adds an arm (it already had: T-740 filed it reading "sixteen" against a real
/// 17). Derive the number instead.
///
/// T-703 moved the SOURCE of that number one step back, to where it belongs. It used to be
/// counted off `SHORTCUTS` — the documentation — on the argument that the T-692 pins hold the
/// table equal to the bindings. True, but circular: a count taken off the docs measures the
/// docs. It is now taken off `keymap_census`, which reads the live listeners, and this pin
/// additionally holds `SHORTCUTS` to the same total, so the circle is closed from the outside.
#[test]
fn the_help_blurb_counts_the_bindings_correctly() {
    let codes: BTreeSet<&str> = crate::editor::panels::help_modal::SHORTCUTS
        .iter()
        .flat_map(|s| s.codes.iter().copied())
        .collect();
    let bound = crate::editor::panels::help_modal::keymap_census::all_bound_codes();
    assert_eq!(
        codes.len(),
        bound.len(),
        "T-740: the help table documents {} distinct codes but the editor binds {} ({bound:?}) \
         — the count in `eden_help`'s header cannot be right about both",
        codes.len(),
        bound.len()
    );
    let word = english(bound.len());
    let sentence = format!("binds {word} distinct `KeyboardEvent` codes");
    assert!(
        include_str!("../panels/help_modal.rs").contains(&sentence),
        "T-669/T-740: the editor now binds {} distinct key codes ({bound:?}), so \
         `eden_help`'s opening paragraph must read \"{sentence}\"",
        bound.len()
    );
}

/// Small-integer spelling. T-703 folded the second copy of this into
/// `keymap_census::spell`, beside the census the number is derived from.
fn english(n: usize) -> String {
    crate::editor::panels::help_modal::keymap_census::spell(n)
}
