//! Role: trigger edits, the closed activation set, and the owner-link line.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

fn document_with_one_trigger() -> MissionDocCore {
    let core = MissionDocCore::new();
    core.add_circle_trigger("t1", "presence", 100.0, 200.0, 50.0);
    core
}

fn activation_of(core: &MissionDocCore, id: &str) -> String {
    trigger_rows(core)
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.id == id)
        .map(|r| r.activation)
        .unwrap_or_default()
}

#[test]
fn an_activation_outside_the_closed_set_never_reaches_the_document() {
    let core = document_with_one_trigger();
    assert!(!set_trigger_activation(&core, "t1", "invented"));
    assert_eq!(activation_of(&core, "t1"), "presence");
    assert!(set_trigger_activation(&core, "t1", "radio"));
    assert_eq!(activation_of(&core, "t1"), "radio");
}

#[test]
fn a_name_is_set_and_an_emptied_field_removes_the_key() {
    let core = document_with_one_trigger();
    set_trigger_name(&core, "t1", Some("Ambush"));
    let named = trigger_rows(&core).unwrap_or_default();
    assert_eq!(named[0].name.as_deref(), Some("Ambush"));

    set_trigger_name(&core, "t1", None);
    let cleared = trigger_rows(&core).unwrap_or_default();
    assert_eq!(cleared[0].name, None);
}

#[test]
fn a_rule_on_a_trigger_that_does_not_exist_is_refused() {
    let core = document_with_one_trigger();
    assert!(!apply_trigger_rule(
        &core,
        "t-missing",
        "radius",
        Some(serde_json::json!(5))
    ));
    assert!(apply_trigger_rule(
        &core,
        "t1",
        "radius",
        Some(serde_json::json!(5))
    ));
    let rows = trigger_rows(&core).unwrap_or_default();
    assert_eq!(rows[0].rules.get("radius"), Some(&serde_json::json!(5)));
}

#[test]
fn deleting_a_trigger_takes_it_out_of_the_count() {
    let core = document_with_one_trigger();
    assert_eq!(trigger_count(&core), 1);
    delete_trigger(&core, "t1");
    assert_eq!(trigger_count(&core), 0);
}

#[test]
fn the_owner_line_needs_a_selection_an_owner_and_a_placed_entity() {
    let core = document_with_one_trigger();
    assert!(owner_line_world(&core, None).is_none());
    assert!(
        owner_line_world(&core, Some("t1")).is_none(),
        "an unowned trigger draws no line"
    );

    set_trigger_owner(&core, "t1", Some("slot-gone"));
    assert!(
        owner_line_world(&core, Some("t1")).is_none(),
        "a dangling owner resolves to nothing rather than to a line"
    );
}
