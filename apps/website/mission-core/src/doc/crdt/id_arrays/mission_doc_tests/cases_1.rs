//! Role: Domain regression cases.
//! Position: `doc/crdt/id_arrays/mission_doc_tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn two_peers_concurrent_slot_id_appends_both_survive() {
    let (a, b) = seed_peers();
    a.add_slot(
        "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
    );
    b.add_slot(
        "from-b", "sq1", "lyr", 1, "Medic", None, None, 30.0, 40.0, 0.0, 0.0,
    );
    a.apply_update(&b.encode_state()).expect("a integrates b");
    b.apply_update(&a.encode_state()).expect("b integrates a");
    let mut ids = json_ids(&a, "sq1");
    ids.sort();
    assert_eq!(
        ids,
        vec!["from-a".to_string(), "from-b".to_string()],
        "concurrent slotIds appends must both survive; got {ids:?}"
    );
    let mut lids = json_layer_ids(&a, "lyr");
    lids.sort();
    assert_eq!(
        lids,
        vec!["from-a".to_string(), "from-b".to_string()],
        "concurrent entityIds appends must both survive; got {lids:?}"
    );
    assert_eq!(json_ids(&a, "sq1"), json_ids(&b, "sq1"));
}

#[test]
fn undo_removes_only_the_local_append() {
    let (mut a, b) = seed_peers();
    a.add_slot(
        "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
    );
    b.add_slot(
        "from-b", "sq1", "lyr", 1, "Medic", None, None, 30.0, 40.0, 0.0, 0.0,
    );
    a.apply_update(&b.encode_state()).expect("a integrates b");
    assert!(a.undo(), "undo local add_slot");
    let mut ids = json_ids(&a, "sq1");
    ids.sort();
    assert_eq!(
        ids,
        vec!["from-b".to_string()],
        "undo must drop only the local id; peer's from-b stays: {ids:?}"
    );
}

#[test]
fn hydrate_legacy_payload_migrates_slot_ids_to_yarray() {
    let doc = MissionDocCore::with_client_id(7);
    doc.hydrate(
            r#"{
                "editor": {
                    "factions": [{"id":"f1","key":"BLUFOR","name":"BLUFOR","squadIds":["sq1"]}],
                    "squads": [{"id":"sq1","factionId":"f1","name":"Alpha","slotIds":["s1","s2"]}],
                    "slots": [
                        {"id":"s1","squadId":"sq1","role":"Rifleman","position":{"x":1,"y":2,"z":0,"rotation":0}},
                        {"id":"s2","squadId":"sq1","role":"Medic","position":{"x":3,"y":4,"z":0,"rotation":0}}
                    ],
                    "editorLayers": [{"id":"lyr","name":"Default","parentId":null,"entityIds":["s1","s2"]}]
                }
            }"#,
            "lyr",
        );
    assert_eq!(json_ids(&doc, "sq1"), ["s1", "s2"]);
    assert_eq!(json_layer_ids(&doc, "lyr"), ["s1", "s2"]);
    assert!(
        id_list_is_native(&doc, "squads", "sq1", SLOT_IDS),
        "hydrate must migrate squad.slotIds to YArray"
    );
    assert!(
        id_list_is_native(&doc, "editorLayers", "lyr", ENTITY_IDS),
        "hydrate must migrate layer.entityIds to YArray"
    );
}
