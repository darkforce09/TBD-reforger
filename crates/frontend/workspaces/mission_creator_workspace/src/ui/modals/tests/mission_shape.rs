use super::{GAME_MODES, PlayerCount, is_known_game_mode};
use crate::ui::docks::top_strip::is_mission_row_id;

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
    assert!(
        PlayerCount {
            placed: 84,
            declared: Some(64)
        }
        .disagrees()
    );
    assert!(
        PlayerCount {
            placed: 0,
            declared: Some(64)
        }
        .disagrees()
    );
    assert!(
        !PlayerCount {
            placed: 64,
            declared: Some(64)
        }
        .disagrees()
    );
    // No row means nothing to disagree WITH — the block shows the derived count alone.
    assert!(
        !PlayerCount {
            placed: 84,
            declared: None
        }
        .disagrees()
    );
}
