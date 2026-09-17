use crate::v2::core::test_support::class_r_scrub::live_source;

/// The six chords this slice binds, as their `KeyboardEvent.code`. Kept here as bare literals
/// (not imported) so the pin still means something if the shared table is renamed out from
/// under it.
const ARRANGE_CODES: [&str; 6] = ["KeyL", "KeyR", "KeyT", "KeyB", "KeyH", "KeyV"];

/// THE DEFECT: `Alt` + each of the six reaches no keydown arm.
#[test]
fn the_editor_keydown_binds_the_arrange_chords() {
    let src = live_source(super::source::raw_editor());
    for code in ARRANGE_CODES {
        let arm = format!("\"{code}\" if !modk && ev.alt_key() && !ev.shift_key() =>");
        assert!(
            src.contains(&arm),
            "T-939.4: the editor keydown has no `{arm}` arm — the Arrange chord is ignored and \
             align / space are reachable only with the mouse"
        );
    }
}

/// The six codes must be the six the shared list keys, and no others. Written against
/// `top_strip::ARRANGE` so the chords cannot be bound here and advertised as something else on
/// the menu row — the failure mode a source pin over this file alone could not see.
#[test]
fn the_bound_codes_are_exactly_the_shared_lists_chorded_rows() {
    let mut from_list: Vec<&str> = crate::v2::apps::editor::ui::docks::top_strip::ARRANGE
        .iter()
        .filter(|e| !e.code.is_empty())
        .map(|e| e.code)
        .collect();
    let mut bound = ARRANGE_CODES.to_vec();
    from_list.sort_unstable();
    bound.sort_unstable();
    assert_eq!(
        from_list, bound,
        "T-939.4: the keydown arms and `top_strip::ARRANGE`'s chorded rows must name the same \
         codes — a chord bound here but not listed there is undiscoverable, and one listed \
         there but not bound here is advertised and dead"
    );
    for entry in crate::v2::apps::editor::ui::docks::top_strip::ARRANGE
        .iter()
        .filter(|e| !e.code.is_empty())
    {
        assert!(
            !entry.chord.is_empty(),
            "T-939.4: `{}` keys `{}` but prints no chord — the row would run on a key nobody \
             can find",
            entry.label,
            entry.code
        );
    }
}

/// **The acceptance line's other half: the chords are INERT on a single selection.**
///
/// `arrange_chord` is wasm-only (it reads the live selection out of `editor_ops`), so this is a
/// source pin like every other keydown contract in this file. It asserts the shape that makes
/// the chord inert rather than the words around it: the helper returns EARLY, before any
/// `run_arrange` call, when the selection is under the shared floor — and because it returns
/// `false` there, the closure's `prevent_default` never runs either, so the key falls through
/// untouched instead of being swallowed by a command that did nothing.
#[test]
fn a_chord_below_the_selection_floor_does_nothing_and_keeps_the_key() {
    let src = live_source(super::source::raw_editor());
    let at = src
        .find("fn arrange_chord(")
        .expect("T-939.4: the shared chord helper");
    let body = &src[at..];
    let end = body.find("\n}").map_or(body.len(), |i| i + 2);
    let body = &body[..end];
    let gate = body
        .find("< top_strip::ARRANGE_MIN_SELECTION")
        .expect("T-939.4: the chord helper must gate on the shared selection floor");
    let ret = body
        .find("return false")
        .expect("T-939.4: the gate must bail rather than fall through");
    let run = body
        .find("top_strip::run_arrange(")
        .expect("T-939.4: the chord must reach the shared invoker");
    assert!(
        gate < ret && ret < run,
        "T-939.4: the selection gate must come BEFORE the early return and the early return \
         BEFORE the invoker — a gate after the call would arrange first and check later. \
         Body:\n{body}"
    );
    assert!(
        body.contains("website_map_engine::editing::host::selection_len()"),
        "T-939.4: the floor must be measured against the LIVE selection, not a mirror that can \
         go stale between a click and a keypress. Body:\n{body}"
    );
}

/// **"Each Arrange chord performs the same operation as its menu entry"**, held structurally.
///
/// Every arm in the chord listener is a call to `arrange_chord`, which calls
/// `top_strip::run_arrange` — the same function `context_menu::dispatch` calls for its
/// `ArrangeRun` row and the same one `top_strip::run_action` routes its four placement arms
/// through. This pin refuses the drift that would break the claim: an arm that reaches
/// `editor_ops` (or anything else) DIRECTLY from this file would be a second implementation,
/// and it is the second implementation that eventually disagrees with the first.
#[test]
fn every_chord_arm_is_a_thin_caller_of_the_shared_invoker() {
    let src = live_source(super::source::raw_editor());
    let at = src
        .find("let arrange = window_event_")
        .expect("T-939.4: the chord listener");
    let body = &src[at..];
    let end = body
        .find("on_cleanup(move || arrange.remove())")
        .expect("T-939.4: the listener's cleanup");
    let body = &body[..end];
    assert_eq!(
        body.matches("arrange_chord(top_strip::ArrangeKind::")
            .count(),
        ARRANGE_CODES.len(),
        "T-939.4: every one of the {} arms must hand off to the shared helper. Body:\n{body}",
        ARRANGE_CODES.len()
    );
    for direct in [
        "align_selection",
        "space_selection",
        "orient_selection",
        "apply_pattern_to_selection",
    ] {
        assert!(
            !body.contains(direct),
            "T-939.4: the keydown must not call `{direct}` itself — placement logic belongs in \
             `top_strip` / `editor_ops`, and a second copy here is how a click and a chord come \
             to mean different things"
        );
    }
    // A chord typed into an attribute field must never rearrange the map. Same guard every
    // other editor chord sits behind.
    assert!(
        body.contains("in_editable_field()"),
        "T-939.4: the chord listener must yield while the operator is typing. Body:\n{body}"
    );
}
