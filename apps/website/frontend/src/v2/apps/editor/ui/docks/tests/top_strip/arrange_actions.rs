//! Arrange actions tests for the top command strip.

/* ═════════ T-939.4 — one Arrange list, three surfaces ═══════════════════════════════════════════
 *
 * The ticket's first requirement is that this file "exposes the Arrange entries as one shared list
 * (label, chord, invoker) consumed by both menus". A list is only shared while nothing else keeps a
 * copy, and the way copies get made is not malice — it is a menu row typed in a hurry beside the
 * one that already exists. These pins make that mistake red instead of invisible.
 */
use super::{
    arrange_chord_for_label, arrange_for_code, MenuAction, ARRANGE, ARRANGE_ITEMS,
    ARRANGE_MIN_SELECTION, MENUS,
};

fn menu_bar_arrange() -> &'static [super::MenuItem] {
    MENUS
        .into_iter()
        .find(|(name, _)| *name == "Arrange")
        .expect("T-939.4: the Arrange menu")
        .1
}

/// The dropdown IS the list. Not "matches" it by a hand-kept parallel table — it is built from
/// it, and this asserts the build did not lose or reorder anything.
#[test]
fn the_menu_bar_renders_the_shared_list_in_order() {
    let rows = menu_bar_arrange();
    assert_eq!(
        rows.len(),
        ARRANGE.len(),
        "T-939.4: the Arrange dropdown and the shared list must be the same length"
    );
    assert!(
        std::ptr::eq(rows.as_ptr(), ARRANGE_ITEMS.as_ptr()),
        "T-939.4: the Arrange menu must point AT the derived array — a fresh slice literal here \
         would be the second copy this whole design exists to prevent"
    );
    for (row, src) in rows.iter().zip(ARRANGE.iter()) {
        assert_eq!(row.label, src.label);
        assert!(
            matches!(
                row.action,
                Some(
                    MenuAction::Pattern(_)
                        | MenuAction::Align(_)
                        | MenuAction::Space(_)
                        | MenuAction::Orient(_)
                )
            ),
            "T-939.4: `{}` must dispatch a placement action, never `None` (a dead row)",
            src.label
        );
    }
}

/// A chord is looked up by CODE and printed by LABEL, and both lookups have to agree with the
/// row they came from — otherwise the menu advertises one key and another one acts.
#[test]
fn code_and_label_lookups_agree_with_the_row() {
    for entry in ARRANGE.iter() {
        assert_eq!(
            arrange_chord_for_label(entry.label),
            entry.chord,
            "T-939.4: `{}`'s printed chord must be its own",
            entry.label
        );
        if entry.code.is_empty() {
            assert!(
                entry.chord.is_empty(),
                "T-939.4: `{}` prints a chord but keys no code — an advertised dead key",
                entry.label
            );
        } else {
            let found = arrange_for_code(entry.code)
                .unwrap_or_else(|| panic!("T-939.4: `{}` is unreachable", entry.code));
            assert_eq!(found.kind, entry.kind);
        }
    }
}

/// The chord-less rows must never be reachable by a keypress. The empty `code` on thirteen of
/// the nineteen is a sentinel, and a lookup that took `""` as a match would make ALL of them
/// answer to the same phantom key — the shape a `find` on an empty needle produces by accident.
#[test]
fn the_empty_code_sentinel_matches_nothing() {
    assert!(arrange_for_code("").is_none());
    assert!(arrange_for_code("KeyQ").is_none());
    let keyed = ARRANGE.iter().filter(|e| !e.code.is_empty()).count();
    assert_eq!(
        keyed, 6,
        "T-939.4: six rows carry chords — the four edge aligns and the two distributes"
    );
}

/// Labels are the join between the two menus (the context submenu renders them, and the strip
/// looks a chord up by them), so a duplicate would make one row shadow another's chord.
#[test]
fn every_label_and_kind_is_unique() {
    let mut labels: Vec<&str> = ARRANGE.iter().map(|e| e.label).collect();
    labels.sort_unstable();
    let n = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), n, "T-939.4: duplicate Arrange label");
    let mut codes: Vec<&str> = ARRANGE
        .iter()
        .map(|e| e.code)
        .filter(|c| !c.is_empty())
        .collect();
    codes.sort_unstable();
    let n = codes.len();
    codes.dedup();
    assert_eq!(codes.len(), n, "T-939.4: two Arrange rows claim one code");
}

/// The floor the context submenu and the chords share. Two, not one: one object cannot be
/// aligned to anything and has no gap to distribute.
#[test]
fn the_selection_floor_is_two() {
    assert_eq!(ARRANGE_MIN_SELECTION, 2);
}

/// The click path and the chord path must be the SAME body, not two that agree today.
/// `run_action`'s four placement arms hand off to `run_arrange_action`, which is what
/// `run_arrange` — the context menu's and the keydown's door — calls. Source-pinned because the
/// dispatch bodies are wasm-only.
#[test]
fn the_menu_click_and_the_chord_share_one_invoker() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_item};
    let code = live_code(include_str!("../../top_strip.rs"));
    // `run_action` is a CLOSURE over the strip's signals, not a free fn — `only_item` slices it
    // from its `let` head all the same, and still refuses a second definition.
    let run_action = only_item(&code, "let run_action = move |a: MenuAction|");
    assert!(
        run_action.contains("run_arrange_action(a)"),
        "T-939.4: the top-strip click must go through the shared invoker"
    );
    for direct in ["align_selection", "space_selection", "orient_selection"] {
        assert!(
            !run_action.contains(direct),
            "T-939.4: `run_action` must not call `{direct}` itself any more — that is the copy \
             the chord would drift away from"
        );
    }
    let invoker = only_item(&code, "pub fn run_arrange(");
    assert!(
        invoker.contains("run_arrange_action("),
        "T-939.4: `run_arrange` must delegate to the one body"
    );
}
