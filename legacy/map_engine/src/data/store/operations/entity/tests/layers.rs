//! Role: layer authoring over a live document.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

fn document_with_one_layer() -> MissionDocCore {
    let core = MissionDocCore::new();
    core.add_editor_layer("layer-a", "Alpha", None);
    core
}

#[test]
fn a_created_folder_nests_under_the_active_one_and_arms_its_rename() {
    let core = document_with_one_layer();
    let id = create_layer(&core, Some("layer-a".to_string()));
    let rows = layer_rows(&core);
    let created = rows.iter().find(|l| l.id == id).expect("the folder exists");
    assert_eq!(created.parent_id.as_deref(), Some("layer-a"));
    assert_eq!(take_rename_armed().as_deref(), Some(id.as_str()));
    assert!(
        take_rename_armed().is_none(),
        "the armed rename is consumed on read"
    );
}

#[test]
fn an_active_folder_the_document_no_longer_holds_creates_a_root() {
    let core = document_with_one_layer();
    let id = create_layer(&core, Some("layer-gone".to_string()));
    let rows = layer_rows(&core);
    let created = rows.iter().find(|l| l.id == id).expect("the folder exists");
    assert_eq!(created.parent_id, None);
}

#[test]
fn a_blank_name_is_refused_and_leaves_the_label_alone() {
    let core = document_with_one_layer();
    assert!(!rename_layer(&core, "layer-a", "   "));
    assert_eq!(layer_rows(&core)[0].name, "Alpha");
    assert!(rename_layer(&core, "layer-a", "  Bravo  "));
    assert_eq!(layer_rows(&core)[0].name, "Bravo");
}

#[test]
fn the_last_folder_is_never_deleted_so_the_document_keeps_a_layer() {
    let core = document_with_one_layer();
    delete_layer(&core, "layer-a");
    assert_eq!(layer_rows(&core).len(), 1);

    core.add_editor_layer("layer-b", "Bravo", None);
    delete_layer(&core, "layer-b");
    let rows = layer_rows(&core);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "layer-a");
}

#[test]
fn reparenting_moves_a_folder_under_another_and_back_to_the_root() {
    let core = document_with_one_layer();
    core.add_editor_layer("layer-b", "Bravo", None);
    reparent_layer(&core, "layer-b", Some("layer-a".to_string()));
    let nested = layer_rows(&core)
        .into_iter()
        .find(|l| l.id == "layer-b")
        .expect("the folder exists");
    assert_eq!(nested.parent_id.as_deref(), Some("layer-a"));

    reparent_layer(&core, "layer-b", None);
    let rooted = layer_rows(&core)
        .into_iter()
        .find(|l| l.id == "layer-b")
        .expect("the folder exists");
    assert_eq!(rooted.parent_id, None);
}

#[test]
fn a_folder_carries_its_own_hidden_flag_on_the_layer_row() {
    let core = document_with_one_layer();
    set_layer_hidden(&core, "layer-a", true);
    assert!(layer_rows(&core)[0].hidden);
    set_layer_hidden(&core, "layer-a", false);
    assert!(!layer_rows(&core)[0].hidden);
}

#[test]
fn the_reveal_all_clears_every_hidden_slot_and_a_second_pass_clears_nothing() {
    let core = document_with_one_layer();
    core.add_squad("squad-a", "faction-BLUFOR", "Alpha", None);
    core.add_slot(
        "slot-1", "squad-a", "layer-a", 0, "RFL", None, None, 0.0, 0.0, 0.0, 0.0,
    );
    core.set_slot_editor_hidden("slot-1", true);
    assert_eq!(show_all_hidden(&core), 1);
    assert_eq!(
        show_all_hidden(&core),
        0,
        "a second reveal clears nothing and is not an edit"
    );
}

#[test]
fn the_active_folder_is_kept_when_live_and_reported_stale_when_gone() {
    let core = document_with_one_layer();
    let live = ensure_layer(
        &core,
        Some("layer-a".to_string()),
        "layer-default",
        "Default",
    );
    assert_eq!(live.layer_id, "layer-a");
    assert!(!live.active_layer_was_stale);

    let stale = ensure_layer(
        &core,
        Some("layer-gone".to_string()),
        "layer-default",
        "Default",
    );
    assert_eq!(stale.layer_id, "layer-a");
    assert!(stale.active_layer_was_stale);
}

#[test]
fn an_empty_document_seeds_the_default_folder_the_caller_names() {
    let core = MissionDocCore::new();
    let seeded = ensure_layer(&core, None, "layer-default", "Default");
    assert_eq!(seeded.layer_id, "layer-default");
    assert!(!seeded.active_layer_was_stale);
    assert_eq!(layer_rows(&core).len(), 1);
}
