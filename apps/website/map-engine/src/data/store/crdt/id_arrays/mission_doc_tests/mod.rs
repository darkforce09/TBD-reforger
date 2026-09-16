//! Role: Module boundary for doc/crdt/id_arrays/mission_doc_tests.
//! Position: `doc/crdt/id_arrays/mission_doc_tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::data::store::MissionDocCore;

use yrs::updates::decoder::Decode;

use yrs::{Doc, Transact, Update};

fn id_list_is_native(doc: &MissionDocCore, root: &str, key: &str, field: &str) -> bool {
    let probe = Doc::with_client_id(0x00EE_EEEE);
    let map = match root {
        "squads" => probe.get_or_insert_map("squads"),
        "editorLayers" => probe.get_or_insert_map("editorLayers"),
        _ => return false,
    };
    {
        let mut txn = probe.transact_mut();
        let update = Update::decode_v1(&doc.encode_state()).expect("decode encode_state");
        txn.apply_update(update).expect("apply encode_state");
    }
    let txn = probe.transact();
    is_native_array(&txn, &map, key, field)
}

fn json_ids(doc: &MissionDocCore, squad: &str) -> Vec<String> {
    let root: serde_json::Value =
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
    root["squadsById"][squad]["slotIds"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn json_layer_ids(doc: &MissionDocCore, layer: &str) -> Vec<String> {
    let root: serde_json::Value =
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
    root["editorLayersById"][layer]["entityIds"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn seed_peers() -> (MissionDocCore, MissionDocCore) {
    let a = MissionDocCore::with_client_id(0x00A1_A1A1);
    let b = MissionDocCore::with_client_id(0x00B2_B2B2);
    a.add_faction("f1", "BLUFOR", "BLUFOR");
    a.add_squad("sq1", "f1", "Alpha", None);
    a.add_editor_layer("lyr", "Default", None);
    b.apply_update(&a.encode_state())
        .expect("b takes a's empty squad");
    (a, b)
}

mod cases_1;
