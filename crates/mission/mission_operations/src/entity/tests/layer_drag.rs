//! Role: the armed layer-tree drag and the armed squad refile, from pick-up to drop.
//! Position: a unit test of `entity::layer_drag` in `mission_operations`.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;
use crate::entity::{begin_refile, cancel_refile, complete_refile_onto_squad};
use crate::projections::layer_rows;

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
        .map(LayerId::into_inner)
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
    core.reparent_editor_layer("layer-b", Some("layer-a".into()));
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
