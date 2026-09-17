use super::{
    game_mode_failure_message, is_known_game_mode, PlayerCount, GAME_MODES,
    PLAYER_COUNT_RULING_NOTE, SLOTS_PLACED_NOTE,
};
use crate::v2::apps::editor::ui::docks::top_strip::is_mission_row_id;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The select's table is the server's enum. `handlers/missions.rs::valid_game_mode` maps exactly
/// `pve_coop` / `pvp` / `zeus` and 400s the rest, so drift here ships a control that can only
/// fail. Every entry also needs a label — an option with a blank face is not a choice.
#[test]
fn game_mode_table_is_the_patch_enum() {
    let values: Vec<&str> = GAME_MODES.iter().map(|(v, _)| *v).collect();
    assert_eq!(values, vec!["pve_coop", "pvp", "zeus"]);
    for (value, label) in GAME_MODES {
        assert!(
            !label.trim().is_empty(),
            "T-694: game mode {value} has no label"
        );
        assert!(is_known_game_mode(value));
    }
    for bogus in ["", "PVP", "coop", "training", "pve"] {
        assert!(
            !is_known_game_mode(bogus),
            "T-694: {bogus:?} is not a game mode the PATCH accepts"
        );
    }
}

/// The row guard. The editor mounts on synthetic ids too (`draft`, the gate's smoke id), where a
/// shape GET/PATCH is a guaranteed failure, so the id must be checked before either goes out.
#[test]
fn row_id_guard_rejects_the_synthetic_editor_ids() {
    assert!(is_mission_row_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
    for not_a_row in [
        "",
        "draft",
        "smoke",
        "3f2504e0-4f89-11d3-9a0c-0305e82c330",   // too short
        "3f2504e0-4f89-11d3-9a0c-0305e82c33011", // too long
        "3f2504e04f8911d39a0c0305e82c3301aaaa",  // right length, no dashes
        "zzzzzzzz-4f89-11d3-9a0c-0305e82c3301",  // not hex
    ] {
        assert!(
            !is_mission_row_id(not_a_row),
            "T-694: {not_a_row:?} must not be treated as a mission row id"
        );
    }
}

/// The disagreement rule: both numbers are reported, and the explanatory note appears **only**
/// when they differ. An author who placed exactly `max_players` slots needs no essay; an author
/// whose two numbers have drifted needs to be told which one the editor goes by.
///
/// **Kept, not rewritten, by T-782.** The ruling changed what the note *says* (the answer, not
/// the argument) and where the declared figure *renders* — it did not change when the sentence
/// is worth showing, so this predicate still holds exactly as T-694 wrote it. What T-782 adds is
/// [`super::PlayerCount::player_figure`], pinned separately: `disagrees` decides whether to
/// explain, `player_figure` decides what is displayed, and conflating the two is how the ruling
/// would get quietly re-opened.
#[test]
fn player_count_flags_disagreement_and_nothing_else() {
    assert!(PlayerCount {
        placed: 84,
        declared: Some(64)
    }
    .disagrees());
    assert!(PlayerCount {
        placed: 0,
        declared: Some(64)
    }
    .disagrees());
    assert!(!PlayerCount {
        placed: 64,
        declared: Some(64)
    }
    .disagrees());
    // No row means nothing to disagree WITH — the block shows the derived count alone.
    assert!(!PlayerCount {
        placed: 84,
        declared: None
    }
    .disagrees());
}

/// Copy pin. [`SLOTS_PLACED_NOTE`] must keep saying that a slot is not a player and that this is
/// not a server limit — that sentence is the whole of the operator's open question, and a later
/// tidy-up that trims it turns an honest report back into an implied guarantee.
#[test]
fn slots_note_refuses_to_claim_a_server_capacity() {
    let note = SLOTS_PLACED_NOTE.to_lowercase();
    assert!(
        note.contains("seat, not a player"),
        "T-694: the slots note must say a slot is a seat, not a player"
    );
    assert!(
        note.contains("server"),
        "T-694: the slots note must say this is not what the server will hold"
    );
    // T-782 renamed the note and replaced its argument with the answer. This assertion is the
    // T-694 one, unchanged and re-aimed at the constant that succeeded it: the ruling settled
    // WHICH figure the editor reports, not whether either is a capacity, so the copy must still
    // refuse to claim one.
    let ruling = PLAYER_COUNT_RULING_NOTE.to_lowercase();
    assert!(
        ruling.contains("neither is enforced"),
        "T-694/T-782: the ruling note must still say neither figure is enforced here"
    );
}

/// A refused PATCH must name the 403 case separately (retrying cannot help a non-author) and must
/// tell the author the control has been put back — because it has.
#[test]
fn refused_game_mode_patch_explains_itself() {
    let forbidden = game_mode_failure_message(&(403, None));
    assert!(forbidden.to_lowercase().contains("author"));
    assert!(forbidden.to_lowercase().contains("put back"));
    // `api_error_message` sentence-cases what the server said, so compare case-insensitively.
    let other = game_mode_failure_message(&(500, Some("boom".into())));
    assert!(
        other.to_lowercase().contains("boom"),
        "T-694: a non-403 must name what the server said, got {other:?}"
    );
    assert!(other.to_lowercase().contains("put back"));
}

/// **The derived count is derived.** The shape section must read the live document's slot count
/// rather than any stored figure. Perturbation this catches: swapping the call for the row's
/// `max_players`, or for a hand-rolled counter.
///
/// This pins where the number is *sourced*. Since T-782 it is only half the claim — a section
/// could source the slot count correctly and still print the declared figure under **Players** —
/// so `t782_player_count_ruling` pins the other half, that the sourced count is the one shown.
#[test]
fn player_count_comes_from_the_document_slot_count() {
    let src = live_code(&super::source::production_source());
    let body = only_body(&src, &format!("fn render{}", "_shape_section"));
    assert!(
        body.contains(&format!("slot{}", "_count")),
        "T-694: the players figure must come from MissionDocCore's slot count"
    );
    assert!(
        body.contains(&format!("doc{}", "_handle")),
        "T-694: the slot count must be read from the live document handle"
    );
}

/// **The absences.** This slice was told not to answer the "what is the real cap?" question, so
/// the section must invent no minimum, clamp nothing to a server limit, and reduce the two
/// figures to neither. Perturbation this catches: a well-meaning `min(128)`, a `min_players`
/// control, or a `max(placed, declared)` that quietly picks a winner.
#[test]
fn shape_section_invents_no_player_limit() {
    let src = live_code(&super::source::production_source());
    let body = only_body(&src, &format!("fn render{}", "_shape_section"));
    for banned in [
        format!("min{}", "_players"),
        "clamp".to_string(),
        ".min(".to_string(),
        ".max(".to_string(),
        "128".to_string(),
    ] {
        assert!(
            !body.contains(&banned),
            "T-694: `{banned}` must not appear in the shape section — the authoritative player \
             cap is an open question and this dialog reports, it does not decide"
        );
    }
    // Whole-file: no `min_players` anywhere. There is no such column, model field or DTO key,
    // and adding one is the migration this ticket was reinterpreted to avoid.
    assert!(
        !src.contains(&format!("min{}", "_players")),
        "T-694: min_players exists nowhere in the platform — do not introduce it here"
    );
}

/// **Game mode is editable after creation** — the gap the ticket names — and it reaches the row
/// by PATCH, not by an `author_env` document write. Perturbation this catches: wiring the select
/// into the document (where nothing would read it) or dropping the mirror call entirely.
#[test]
fn game_mode_select_patches_the_missions_row() {
    let src = live_code(&super::source::production_source());
    let setter = format!("set{}", "_game_mode");

    // (a) the section's control calls the setter and offers the table's options.
    let body = only_body(&src, &format!("fn render{}", "_shape_section"));
    assert!(
        body.contains(&format!("{setter}(")),
        "T-694: the game mode select must call {setter}"
    );
    assert!(
        body.contains(&format!("GAME{}", "_MODES")),
        "T-694: the options must come from the shared game-mode table"
    );
    // (b) it is a ROW write, not a document write: the row half of this dialog must never reach
    // for the env gate, or the change would land somewhere no compile reads.
    assert!(
        !body.contains(&format!("author{}", "_env")),
        "T-694: game mode is a `missions` row column, not a meta.environment key"
    );

    // (c) the setter itself PATCHes /missions/:id with the game_mode column.
    let setter_body = only_body(&src, &format!("fn {setter}"));
    assert!(
        setter_body.contains(&format!("api{}", "_patch")),
        "T-694: {setter} must PATCH the mission row"
    );
    assert!(
        setter_body.contains(&format!("is{}", "_mission_row_id")),
        "T-694: {setter} must refuse synthetic editor ids before hitting the wire"
    );
}
