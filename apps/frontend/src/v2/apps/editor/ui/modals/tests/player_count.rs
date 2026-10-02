use super::{PlayerCount, MAX_PLAYERS_KEPT_NOTE, PLAYER_COUNT_RULING_NOTE};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// **The ruling, as a value.** Whatever the row declares, the figure is the count of placed
/// slots. Perturbation this catches: pointing the display back at `max_players` — a
/// `player_figure` that prefers `declared` when the row has one, or that treats an empty
/// document as "no answer yet" and borrows the declared cap.
#[test]
fn the_player_figure_is_the_derived_slot_count() {
    for declared in [None, Some(0), Some(1), Some(64), Some(999)] {
        let counts = PlayerCount {
            placed: 84,
            declared,
        };
        assert_eq!(
            counts.player_figure(),
            84,
            "T-782: the editor's player figure is the placed slot count — declared={declared:?} \
             must not change it"
        );
    }
    // The empty document is the case a fallback would look reasonable in, and the case where it
    // would lie hardest: a mission with no slots seats nobody, whatever was typed at creation.
    assert_eq!(
        PlayerCount {
            placed: 0,
            declared: Some(64)
        }
        .player_figure(),
        0,
        "T-782: a mission with no slots placed shows 0, not the declared 64"
    );
}

/// **The seam is a seam.** `player_figure` is the whole of the decision, so its body must be the
/// whole of the decision: `self.placed`, nothing else. An exact-body pin rather than a scoped
/// "must not contain `declared`", because the interesting perturbation is not typing the word —
/// it is calling something that reaches the row on this function's behalf.
#[test]
fn the_seam_reads_the_document_count_and_nothing_else() {
    let src = live_code(&super::source::production_source());
    let seam = format!("fn player{}", "_figure");
    let body: String = only_body(&src, &seam).split_whitespace().collect();
    assert_eq!(
        body, "self.placed",
        "T-782: `{seam}` must be exactly the derived count. Anything else — a fallback to the \
         declared cap, a clamp, a helper that can reach the row — re-opens a question the \
         operator closed, in the one place a reader would not think to look"
    );
}

/// **What is displayed is that derived count.** The section must render the figure through the
/// seam, and no rendered number may bypass it. The negative half is whole-file on purpose (the
/// house rule): a bypass moved into a neighbouring helper is still a bypass.
#[test]
fn the_displayed_players_figure_goes_through_the_seam() {
    let src = live_code(&super::source::production_source());
    let body = only_body(&src, &format!("fn render{}", "_shape_section"));
    assert!(
        body.contains(&format!("player{}()", "_figure")),
        "T-782: the Players cell must render PlayerCount::player_figure() — a section that \
         prints a local it computed itself is a second answer to the ruled question"
    );
    assert!(
        !src.contains(&format!("placed{}", ".to_string()")),
        "T-782: the placed local must not be rendered directly; it reaches the screen through \
         the seam or not at all"
    );
    // The nomination has to be visible, not merely true. Two identically-styled numbers under
    // one heading was the rejected presentation, so the figure carries its own type scale and
    // the second number carries a label saying what it is. Read through the literal-preserving
    // scrub: `live_code` blanks class strings and copy, so these needles are invisible to it.
    // (Stated positively — a negative needle here could not be whole-file, because the same
    // two-column grid is a legitimate layout for Time and Weather further up this file.)
    let lit = live_source(&super::source::production_source());
    let lit_body = only_body(&lit, &format!("fn render{}", "_shape_section"));
    assert!(
        lit_body.contains(&format!("text-headline{}", "-sm")),
        "T-782: the nominated figure must be set apart from the read-only cap cell, not styled \
         as its twin"
    );
    assert!(
        lit_body.contains("declared at creation"),
        "T-782: the second number must be labelled as the author's declaration"
    );
}

/// **`max_players` survived the ruling.** The decision was about what the editor DISPLAYS; the
/// compile path still copies this column into the compiled mission
/// (`dto::MissionDetail::compiled_meta`) and the library card still advertises it. So it must
/// stay read, stay rendered, and stay labelled as the author's declaration — the failure this
/// catches is a tidy-up that reads "the derived count won, delete the loser" and silently drops
/// a value the compiler still ships.
#[test]
fn the_declared_cap_is_kept_and_labelled() {
    let src = live_code(&super::source::production_source());
    assert!(
        src.contains(&format!("max{}", "_players")),
        "T-782: `max_players` reaches the compiled mission — the display ruling must not delete it"
    );
    let body = only_body(&src, &format!("fn render{}", "_shape_section"));
    assert!(
        body.contains("declared"),
        "T-782: the section must still report the author-declared figure"
    );
    let kept = MAX_PLAYERS_KEPT_NOTE.to_lowercase();
    assert!(
        kept.contains("compiled"),
        "T-782: the kept-cap note must say where this figure actually goes, got {kept:?}"
    );
    assert!(
        kept.contains("declared"),
        "T-782: the kept-cap note must say the figure was declared, not counted, got {kept:?}"
    );
}

/// **The note states the decision, not the argument.** The retired copy ended "so both are
/// shown" — an honest sentence while the question was open and a false one now. Its replacement
/// has to name the figure the editor goes by, or the screen still refuses to answer.
#[test]
fn the_ruling_note_names_the_answer_instead_of_the_argument() {
    let note = PLAYER_COUNT_RULING_NOTE.to_lowercase();
    assert!(
        note.contains("goes by the slots placed"),
        "T-782: the note must say which figure the editor goes by, got {note:?}"
    );
    assert!(
        !note.contains("both are shown"),
        "T-782: the disagree-note's refusal to choose was retired with the question, got {note:?}"
    );
    // The reserved sub-question is the operator's: nothing on screen may imply the editor has
    // settled what the COMPILE path does with `max_players`.
    for overreach in ["will be derived", "ignore this", "no longer used"] {
        assert!(
            !note.contains(overreach),
            "T-782: {overreach:?} decides the compile-path question the ruling left open"
        );
    }
}
