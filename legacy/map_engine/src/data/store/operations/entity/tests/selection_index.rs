//! Role: the selected entities read off the document index.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn an_empty_selection_indexes_nothing() {
    let core = MissionDocCore::new();
    core.add_editor_layer("layer-a", "Alpha", None);
    assert!(selection_entities(&core, &[]).is_empty());
}

#[test]
fn only_the_selected_ids_come_back_and_an_unknown_id_matches_nothing() {
    let core = MissionDocCore::new();
    core.add_editor_layer("layer-a", "Alpha", None);
    core.add_editor_layer("layer-b", "Bravo", None);

    let picked = selection_entities(&core, &["layer-b".to_string()]);
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].id, "layer-b");

    assert!(selection_entities(&core, &["nothing-here".to_string()]).is_empty());
}
