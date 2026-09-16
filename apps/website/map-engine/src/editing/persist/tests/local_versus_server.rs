//! Role: which of the three verdicts each pair of documents earns.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents built in the test body; nothing shared between tests.
//! Invariants: an identical pair never prompts however it was produced, a genuinely different pair
//! always does, and a field the mission row owns — terrain, time, weather — is never mistaken for
//! authored divergence.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use serde_json::json;

/// A payload the editor would save: one slot per id, on the default layer.
fn payload(slot_ids: &[&str]) -> serde_json::Value {
    let slots: Vec<serde_json::Value> = slot_ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            json!({
                "id": id,
                "x": 100.0 + i as f64,
                "y": 0.0,
                "z": 200.0 + i as f64,
                "rotation": 0.0,
                "layerId": DEFAULT_LAYER_ID,
            })
        })
        .collect();
    json!({ "title": "Fixture", "editor": { "slots": slots } })
}

/// The document a hydrate of `server` produces — the same document an adopt would leave behind.
fn document_of(server: &serde_json::Value) -> DocHandle {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.hydrate(&server.to_string(), DEFAULT_LAYER_ID);
    core.set_origin_init(false);
    Rc::new(RefCell::new(Some(core)))
}

fn classify(doc: &DocHandle, server: &serde_json::Value) -> Option<LocalDraftVerdict> {
    classify_local_draft(doc, server, &server.to_string())
}

#[test]
fn a_document_with_nothing_authored_in_it_is_empty_rather_than_divergent() {
    let doc: DocHandle = Rc::new(RefCell::new(Some(MissionDocCore::new())));
    assert_eq!(
        classify(&doc, &payload(&["slot-1"])),
        Some(LocalDraftVerdict::Empty)
    );
}

/// The case a version marker gets wrong in the "prompt when there is nothing to choose" direction.
#[test]
fn a_local_draft_the_adopt_would_reproduce_matches_and_never_prompts() {
    let server = payload(&["slot-1", "slot-2", "slot-3"]);
    let doc = document_of(&server);
    assert_eq!(
        classify(&doc, &server),
        Some(LocalDraftVerdict::MatchesServer)
    );
}

/// The warm reopen: a draft replayed out of its own stored record still matches the version it was
/// hydrated from. Nothing about the session that produced the document takes part in the verdict.
#[test]
fn a_draft_replayed_from_its_own_record_still_matches_the_server() {
    let server = payload(&["slot-1", "slot-2", "slot-3"]);
    let bytes = document_of(&server)
        .borrow()
        .as_ref()
        .expect("a document")
        .encode_state();
    let restored = MissionDocCore::new();
    restored.set_origin_init(true);
    restored.apply_update(&bytes).expect("the record replays");
    restored.set_origin_init(false);
    let doc: DocHandle = Rc::new(RefCell::new(Some(restored)));
    assert_eq!(
        classify(&doc, &server),
        Some(LocalDraftVerdict::MatchesServer)
    );
}

#[test]
fn one_more_slot_locally_is_a_divergence_settled_without_compiling() {
    let doc = document_of(&payload(&["slot-1", "slot-2"]));
    assert_eq!(
        classify(&doc, &payload(&["slot-1"])),
        Some(LocalDraftVerdict::Diverged)
    );
    let doc = document_of(&payload(&["slot-1"]));
    assert_eq!(
        classify(&doc, &payload(&["slot-1", "slot-2"])),
        Some(LocalDraftVerdict::Diverged)
    );
}

/// The same count with different content: the cheap tiers cannot settle it, so the compile does.
#[test]
fn the_same_number_of_different_slots_is_still_a_divergence() {
    let doc = document_of(&payload(&["slot-1", "slot-2"]));
    assert_eq!(
        classify(&doc, &payload(&["slot-1", "slot-9"])),
        Some(LocalDraftVerdict::Diverged)
    );
}

/// Terrain, time and weather are mission-ROW fields, written into the document after every hydrate.
/// Treating them as authored content would turn a changed weather dropdown into a data-loss prompt.
#[test]
fn a_row_field_the_editor_does_not_author_is_not_a_divergence() {
    let server = payload(&["slot-1", "slot-2"]);
    let doc = document_of(&server);
    {
        let guard = doc.borrow();
        let core = guard.as_ref().expect("a document");
        core.apply_row_meta(
            "Fixture",
            "arland",
            Some("18:30:00".to_string()),
            Some("Storm".to_string()),
            Some("A blurb the payload never carried".to_string()),
        );
    }
    assert_eq!(
        classify(&doc, &server),
        Some(LocalDraftVerdict::MatchesServer),
        "the row's own fields are supplied separately and must not raise a prompt"
    );
}

#[test]
fn there_is_no_verdict_without_a_document() {
    let doc: DocHandle = Rc::new(RefCell::new(None));
    assert_eq!(classify(&doc, &payload(&["slot-1"])), None);
}

#[test]
fn a_payload_with_no_slots_at_all_counts_none() {
    assert_eq!(server_slot_count(&json!({})), 0);
    assert_eq!(server_slot_count(&json!({"editor": {}})), 0);
    assert_eq!(server_slot_count(&json!({"editor": {"slots": "nope"}})), 0);
    assert_eq!(server_slot_count(&payload(&["a", "b", "c"])), 3);
}

#[test]
fn only_the_authored_keys_take_part_in_the_comparison() {
    let a = json!({"editor": {"slots": []}, "map": {"terrain": "everon"}, "schemaVersion": 1});
    let b = json!({"editor": {"slots": []}, "map": {"terrain": "arland"}, "schemaVersion": 2});
    assert!(same_authored_content(&a, &b));

    let c = json!({"editor": {"slots": [{"id": "x"}]}, "map": {"terrain": "everon"}});
    assert!(!same_authored_content(&a, &c));

    for key in ["loadouts", "objectives", "vehicles", "markers"] {
        let mut differs = a.clone();
        differs[key] = json!([{ "id": "one" }]);
        assert!(
            !same_authored_content(&a, &differs),
            "{key} is authored content and a difference in it must be seen"
        );
    }
}
