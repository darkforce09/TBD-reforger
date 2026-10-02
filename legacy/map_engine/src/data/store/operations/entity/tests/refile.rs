//! Role: the armed squad refile, from pick-up to drop.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_refile_moves_the_armed_slot_into_the_squad_it_is_dropped_on() {
    let core = MissionDocCore::new();
    core.add_squad("squad-a", "faction-BLUFOR", "Alpha", None);
    core.add_squad("squad-b", "faction-BLUFOR", "Bravo", None);
    core.add_slot(
        "slot-1", "squad-a", "layer-a", 0, "RFL", None, None, 0.0, 0.0, 0.0, 0.0,
    );

    begin_refile("slot-1".to_string());
    assert!(complete_refile_onto_squad(&core, "squad-b"));
    assert!(
        !complete_refile_onto_squad(&core, "squad-a"),
        "the drop consumed the armed refile"
    );
}

#[test]
fn a_cancelled_refile_is_not_a_drop() {
    let core = MissionDocCore::new();
    begin_refile("slot-1".to_string());
    cancel_refile();
    assert!(!complete_refile_onto_squad(&core, "squad-b"));
}
