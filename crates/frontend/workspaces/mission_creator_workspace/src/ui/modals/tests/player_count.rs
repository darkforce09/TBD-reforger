use super::PlayerCount;

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
