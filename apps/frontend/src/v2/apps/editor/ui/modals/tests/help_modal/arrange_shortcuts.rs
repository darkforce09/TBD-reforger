use super::{GROUPS, SHORTCUTS};
use crate::v2::apps::editor::ui::docks::top_strip::ARRANGE;

/// Every chorded Arrange row has a help row filed under `Arrange` whose chord text CONTAINS the
/// spelling the menus print. `contains` rather than equality because a row may pair two chords
/// (`"Alt + L  /  Alt + R"`) — pairing is a presentation choice, printing a different chord is
/// not.
#[test]
fn arrange_help_rows_match_the_shared_list() {
    let arrange_rows: Vec<&super::Shortcut> =
        SHORTCUTS.iter().filter(|s| s.group == "Arrange").collect();
    assert!(
        !arrange_rows.is_empty(),
        "T-939.4: the Controls Hint must carry an Arrange section"
    );
    assert!(
        GROUPS.contains(&"Arrange"),
        "T-939.4: `Arrange` must be a rendered heading, or every row above is invisible"
    );
    for entry in ARRANGE.iter().filter(|e| !e.code.is_empty()) {
        let row = arrange_rows
            .iter()
            .find(|s| s.codes.contains(&entry.code))
            .unwrap_or_else(|| {
                panic!(
                    "T-939.4: `{}` binds `{}` and the Arrange section documents no row for it",
                    entry.label, entry.code
                )
            });
        assert!(
            row.chord.contains(entry.chord),
            "T-939.4: the help card spells `{}`'s chord `{}`, the menus print `{}` — one of \
             them is lying to the operator",
            entry.label,
            row.chord,
            entry.chord
        );
    }
}

/// The other direction: the Arrange section must not document a code the shared list does not
/// key. `no_help_entry_invents_a_binding` catches a code nothing in the EDITOR binds; this
/// catches the narrower rot of an Arrange row surviving after its chord moved off the list.
#[test]
fn the_arrange_section_documents_no_chord_the_list_dropped() {
    let keyed: Vec<&str> = ARRANGE
        .iter()
        .filter(|e| !e.code.is_empty())
        .map(|e| e.code)
        .collect();
    for row in SHORTCUTS.iter().filter(|s| s.group == "Arrange") {
        for code in row.codes {
            assert!(
                keyed.contains(code),
                "T-939.4: the Arrange section documents `{code}`, which `top_strip::ARRANGE` \
                 no longer keys — drop the row or restore the chord"
            );
        }
    }
}
