//! Role: the armed layer-tree drag, from pick-up to drop.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;
use crate::data::store::operations::projections::layer_rows;

fn two_folders() -> MissionDocCore {
    let core = MissionDocCore::new();
    core.add_editor_layer("layer-a", "Alpha", None);
    core.add_editor_layer("layer-b", "Bravo", None);
    core
}

fn parent_of(core: &MissionDocCore, id: &str) -> Option<String> {
    layer_rows(core)
        .into_iter()
        .find(|l| l.id == id)
        .and_then(|l| l.parent_id)
}

#[test]
fn a_folder_drag_reparents_onto_the_folder_it_is_dropped_on() {
    let core = two_folders();
    begin_layer_drag("layer-b".to_string());
    assert!(complete_layer_drop_onto_folder(&core, "layer-a"));
    assert_eq!(parent_of(&core, "layer-b").as_deref(), Some("layer-a"));
}

#[test]
fn dropping_a_folder_on_itself_is_refused_and_consumes_the_drag() {
    let core = two_folders();
    begin_layer_drag("layer-a".to_string());
    assert!(!complete_layer_drop_onto_folder(&core, "layer-a"));
    assert!(
        !complete_layer_drop_onto_folder(&core, "layer-b"),
        "the refused drop still consumed the armed drag"
    );
}

#[test]
fn a_cancelled_drag_leaves_the_document_alone() {
    let core = two_folders();
    begin_layer_drag("layer-b".to_string());
    cancel_layer_drag();
    assert!(!complete_layer_drop_onto_folder(&core, "layer-a"));
    assert_eq!(parent_of(&core, "layer-b"), None);
}

#[test]
fn the_root_dropzone_takes_a_folder_and_drops_a_slot() {
    let core = two_folders();
    core.reparent_editor_layer("layer-b", Some("layer-a".to_string()));
    begin_layer_drag("layer-b".to_string());
    assert!(complete_layer_drop_onto_root(&core));
    assert_eq!(parent_of(&core, "layer-b"), None);

    begin_layer_slot_drag("slot-1".to_string());
    assert!(
        !complete_layer_drop_onto_root(&core),
        "a slot has no home outside a folder, so the root dropzone drops it"
    );
}

#[test]
fn nothing_armed_is_not_a_drop() {
    let core = two_folders();
    assert!(!complete_layer_drop_onto_folder(&core, "layer-a"));
    assert!(!complete_layer_drop_onto_root(&core));
}
