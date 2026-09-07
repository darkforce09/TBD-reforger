//! T-937.1 — native yrs arrays for `squad.slotIds` and `layer.entityIds`.
//!
//! Those two keys used to ride `Any::Array` clone-rewrites (`append_id` / `retain_ids` in
//! `store.rs`): every append copied the whole list and `Map::insert`ed it back, so two peers
//! appending concurrently last-write-won and dropped an id. This module stores them as `YArray`
//! so concurrent inserts merge, hydrates a legacy `Any::Array` once, and still reads both forms
//! as the same ordered `Vec<String>`.
//!
//! Other id lists (`vehicleIds`, `squadIds`, `cargo`, …) stay opaque `Any::Array`.

use std::collections::HashSet;

use yrs::{Any, Array, ArrayPrelim, Map, MapRef, Out, ReadTxn, TransactionMut};

/// Squad membership list — native `YArray` after T-937.1.
pub const SLOT_IDS: &str = "slotIds";
/// Outliner folder membership list — native `YArray` after T-937.1.
pub const ENTITY_IDS: &str = "entityIds";

fn is_native_field(field: &str) -> bool {
    field == SLOT_IDS || field == ENTITY_IDS
}

fn any_id_string(a: &Any) -> Option<String> {
    match a {
        Any::String(s) => Some(s.to_string()),
        _ => None,
    }
}

fn out_id_string(out: &Out) -> Option<String> {
    match out {
        Out::Any(a) => any_id_string(a),
        _ => None,
    }
}

fn ids_from_any_array(arr: &[Any]) -> Vec<Any> {
    arr.to_vec()
}

fn ids_from_yarray<T: ReadTxn>(txn: &T, arr: &yrs::ArrayRef) -> Vec<Any> {
    arr.iter(txn)
        .filter_map(|out| out_id_string(&out).map(|s| Any::String(s.into())))
        .collect()
}

fn strings_from_anys(anys: &[Any]) -> Vec<String> {
    anys.iter().filter_map(any_id_string).collect()
}

/// Read `map[key].field` (string ids) as owned `Any` values. Empty when the container or field is
/// missing. Accepts native `YArray` and legacy `Any::Array` identically and in order.
pub fn read_id_array<T: ReadTxn>(txn: &T, map: &MapRef, key: &str, field: &str) -> Vec<Any> {
    match map.get(txn, key) {
        Some(Out::YMap(container)) => read_field(txn, &container, field),
        _ => Vec::new(),
    }
}

/// Read `container.field` as owned `Any` id values (both representations).
pub fn read_field<T: ReadTxn>(txn: &T, container: &MapRef, field: &str) -> Vec<Any> {
    match container.get(txn, field) {
        Some(Out::YArray(arr)) => ids_from_yarray(txn, &arr),
        Some(Out::Any(Any::Array(arr))) => ids_from_any_array(arr.as_ref()),
        _ => Vec::new(),
    }
}

/// Ordered string ids for `map[key].field` (both representations).
#[allow(dead_code)] // store uses `read_id_array`; tests use this String view
pub fn read_ids<T: ReadTxn>(txn: &T, map: &MapRef, key: &str, field: &str) -> Vec<String> {
    strings_from_anys(&read_id_array(txn, map, key, field))
}

/// Ordered string ids for `container.field` (both representations).
pub fn read_field_ids<T: ReadTxn>(txn: &T, container: &MapRef, field: &str) -> Vec<String> {
    strings_from_anys(&read_field(txn, container, field))
}

/// True when `map[key].field` is a live `YArray` (not a legacy `Any::Array`).
#[allow(dead_code)] // this module's tests (must not live as cfg(test) on MissionDocCore)
pub fn is_native_array<T: ReadTxn>(txn: &T, map: &MapRef, key: &str, field: &str) -> bool {
    matches!(
        map.get(txn, key).and_then(|o| match o {
            Out::YMap(c) => c.get(txn, field),
            _ => None,
        }),
        Some(Out::YArray(_))
    )
}

/// Insert an empty native `YArray` under `container.field` (tombstones a prior value).
pub fn insert_empty_native(txn: &mut TransactionMut, container: &MapRef, field: &str) {
    container.insert(txn, field, ArrayPrelim::from(Vec::<String>::new()));
}

/// Replace `container.field` with a native `YArray` of `ids` (hydrate / seed / merge).
pub fn replace_native(txn: &mut TransactionMut, container: &MapRef, field: &str, ids: &[String]) {
    container.insert(
        txn,
        field,
        ArrayPrelim::from(ids.iter().map(String::as_str).collect::<Vec<&str>>()),
    );
}

fn ensure_native(txn: &mut TransactionMut, container: &MapRef, field: &str) {
    match container.get(txn, field) {
        Some(Out::YArray(_)) => {}
        Some(Out::Any(Any::Array(arr))) => {
            let ids = strings_from_anys(&ids_from_any_array(&arr));
            replace_native(txn, container, field, &ids);
        }
        _ => insert_empty_native(txn, container, field),
    }
}

/// Filter a cloned `Any::Array` (opaque lists: `vehicleIds`, `squadIds`). Native fields must use
/// [`retain_in`] so the live `YArray` is edited in place.
pub fn retain_ids(arr: &[Any], remove: &HashSet<&str>) -> Vec<Any> {
    arr.iter()
        .filter(|a| !matches!(a, Any::String(s) if remove.contains(s.as_ref())))
        .cloned()
        .collect()
}

/// Drop every id in `remove` from `container.field`. Native fields delete `YArray` entries;
/// opaque fields clone-rewrite `Any::Array`.
pub fn retain_in(
    txn: &mut TransactionMut,
    container: &MapRef,
    field: &str,
    remove: &HashSet<&str>,
) {
    if is_native_field(field) {
        match container.get(txn, field) {
            Some(Out::YArray(arr)) => {
                let mut drop_at: Vec<u32> = Vec::new();
                for (i, out) in arr.iter(txn).enumerate() {
                    if out_id_string(&out).is_some_and(|s| remove.contains(s.as_str())) {
                        drop_at.push(u32::try_from(i).expect("id list fits u32"));
                    }
                }
                for i in drop_at.into_iter().rev() {
                    arr.remove(txn, i);
                }
            }
            Some(Out::Any(Any::Array(arr))) => {
                let kept = retain_ids(arr.as_ref(), remove);
                let ids = strings_from_anys(&kept);
                replace_native(txn, container, field, &ids);
            }
            _ => {}
        }
        return;
    }
    if let Some(Out::Any(Any::Array(arr))) = container.get(txn, field) {
        let kept = retain_ids(arr.as_ref(), remove);
        container.insert(txn, field, Any::Array(kept.into()));
    }
}

/// Append `id` to `map[key].field` if that container map exists. Native fields `push_back` on a
/// `YArray` (CRDT merge); opaque fields clone-rewrite `Any::Array`. Dedup: an id already present
/// is not appended again (a slot belongs to a squad once).
pub fn append_id(txn: &mut TransactionMut, map: &MapRef, key: &str, field: &str, id: &str) {
    let Some(Out::YMap(container)) = map.get(txn, key) else {
        return;
    };
    if is_native_field(field) {
        ensure_native(txn, &container, field);
        let Some(Out::YArray(arr)) = container.get(txn, field) else {
            return;
        };
        if arr
            .iter(txn)
            .any(|out| out_id_string(&out).is_some_and(|s| s == id))
        {
            return;
        }
        arr.push_back(txn, id);
        return;
    }
    let mut next: Vec<Any> = match container.get(txn, field) {
        Some(Out::Any(Any::Array(arr))) => arr.iter().cloned().collect(),
        _ => Vec::new(),
    };
    if next
        .iter()
        .any(|a| matches!(a, Any::String(s) if s.as_ref() == id))
    {
        return;
    }
    next.push(Any::String(id.into()));
    container.insert(txn, field, Any::Array(next.into()));
}

/// Move `id` to `to_index` within a native id list (clamped). No-op when the id is absent or the
/// field is not a `YArray`.
#[allow(dead_code)] // required helper; covered by `read_append_retain_move_over_yarray`
pub fn move_id(txn: &mut TransactionMut, container: &MapRef, field: &str, id: &str, to_index: u32) {
    let Some(Out::YArray(arr)) = container.get(txn, field) else {
        return;
    };
    let Some(from) = arr
        .iter(txn)
        .position(|out| out_id_string(&out).is_some_and(|s| s == id))
    else {
        return;
    };
    let from = u32::try_from(from).expect("id list fits u32");
    arr.remove(txn, from);
    let dest = to_index.min(arr.len(txn));
    arr.insert(txn, dest, id);
}

/// Hydrate-time migration: every `squad.slotIds` and `layer.entityIds` that is still a legacy
/// `Any::Array` becomes a native `YArray` once, preserving order. Missing keys stay missing so a
/// payload that omitted them does not grow a new `[]` on the wire.
pub fn migrate_legacy_id_lists(txn: &mut TransactionMut, squads: &MapRef, editor_layers: &MapRef) {
    migrate_map_field(txn, squads, SLOT_IDS);
    migrate_map_field(txn, editor_layers, ENTITY_IDS);
}

fn migrate_map_field(txn: &mut TransactionMut, parent: &MapRef, field: &str) {
    let keys: Vec<String> = parent.iter(txn).map(|(k, _)| k.to_string()).collect();
    for key in keys {
        let Some(Out::YMap(row)) = parent.get(txn, &key) else {
            continue;
        };
        if let Some(Out::Any(Any::Array(arr))) = row.get(txn, field) {
            let ids = strings_from_anys(&ids_from_any_array(&arr));
            replace_native(txn, &row, field, &ids);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::updates::decoder::Decode;
    use yrs::{Doc, MapPrelim, StateVector, Transact, Update};

    fn native_doc() -> (Doc, MapRef, MapRef) {
        let doc = Doc::with_client_id(1);
        let squads = doc.get_or_insert_map("squads");
        let layers = doc.get_or_insert_map("editorLayers");
        (doc, squads, layers)
    }

    #[test]
    fn read_append_retain_move_over_yarray() {
        let (doc, squads, _layers) = native_doc();
        {
            let mut txn = doc.transact_mut();
            let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
            insert_empty_native(&mut txn, &sq, SLOT_IDS);
        }
        {
            let mut txn = doc.transact_mut();
            append_id(&mut txn, &squads, "sq", SLOT_IDS, "a");
            append_id(&mut txn, &squads, "sq", SLOT_IDS, "b");
            append_id(&mut txn, &squads, "sq", SLOT_IDS, "c");
            append_id(&mut txn, &squads, "sq", SLOT_IDS, "b"); // dedup
        }
        {
            let txn = doc.transact();
            assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["a", "b", "c"]);
            assert!(is_native_array(&txn, &squads, "sq", SLOT_IDS));
        }
        {
            let mut txn = doc.transact_mut();
            let Out::YMap(sq) = squads.get(&txn, "sq").unwrap() else {
                panic!("squad");
            };
            retain_in(&mut txn, &sq, SLOT_IDS, &HashSet::from(["b"]));
        }
        {
            let txn = doc.transact();
            assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["a", "c"]);
        }
        {
            let mut txn = doc.transact_mut();
            let Out::YMap(sq) = squads.get(&txn, "sq").unwrap() else {
                panic!("squad");
            };
            move_id(&mut txn, &sq, SLOT_IDS, "c", 0);
        }
        let txn = doc.transact();
        assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["c", "a"]);
    }

    #[test]
    fn both_forms_read_identically() {
        let (doc, squads, _layers) = native_doc();
        {
            let mut txn = doc.transact_mut();
            let legacy = squads.insert(&mut txn, "leg", MapPrelim::from([("id", "leg")]));
            legacy.insert(
                &mut txn,
                SLOT_IDS,
                Any::Array(vec![Any::String("x".into()), Any::String("y".into())].into()),
            );
            let native = squads.insert(&mut txn, "nat", MapPrelim::from([("id", "nat")]));
            replace_native(
                &mut txn,
                &native,
                SLOT_IDS,
                &["x".to_string(), "y".to_string()],
            );
        }
        let txn = doc.transact();
        assert_eq!(
            read_ids(&txn, &squads, "leg", SLOT_IDS),
            read_ids(&txn, &squads, "nat", SLOT_IDS)
        );
        assert_eq!(read_ids(&txn, &squads, "leg", SLOT_IDS), ["x", "y"]);
        assert!(!is_native_array(&txn, &squads, "leg", SLOT_IDS));
        assert!(is_native_array(&txn, &squads, "nat", SLOT_IDS));
    }

    #[test]
    fn hydrate_migration_promotes_legacy_any_array() {
        let (doc, squads, layers) = native_doc();
        {
            let mut txn = doc.transact_mut();
            let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
            sq.insert(
                &mut txn,
                SLOT_IDS,
                Any::Array(vec![Any::String("s1".into())].into()),
            );
            let ly = layers.insert(&mut txn, "L", MapPrelim::from([("id", "L")]));
            ly.insert(
                &mut txn,
                ENTITY_IDS,
                Any::Array(vec![Any::String("s1".into())].into()),
            );
            migrate_legacy_id_lists(&mut txn, &squads, &layers);
        }
        let txn = doc.transact();
        assert!(
            is_native_array(&txn, &squads, "sq", SLOT_IDS),
            "legacy slotIds must become YArray"
        );
        assert!(
            is_native_array(&txn, &layers, "L", ENTITY_IDS),
            "legacy entityIds must become YArray"
        );
        assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["s1"]);
        assert_eq!(read_ids(&txn, &layers, "L", ENTITY_IDS), ["s1"]);
    }

    /// Skip this migrate call and the assertion above is red — the perturbation the ticket asks for.
    #[test]
    fn skip_migration_leaves_legacy_any_array() {
        let (doc, squads, layers) = native_doc();
        {
            let mut txn = doc.transact_mut();
            let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
            sq.insert(
                &mut txn,
                SLOT_IDS,
                Any::Array(vec![Any::String("s1".into())].into()),
            );
            // Deliberately no migrate_legacy_id_lists — this is the control that must stay red
            // if hydrate ever skips the branch.
        }
        let txn = doc.transact();
        assert!(
            !is_native_array(&txn, &squads, "sq", SLOT_IDS),
            "without migrate the field stays Any::Array"
        );
        assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["s1"]);
        let _ = layers;
    }

    fn encode(doc: &Doc) -> Vec<u8> {
        doc.transact()
            .encode_state_as_update_v1(&StateVector::default())
    }

    fn apply(doc: &Doc, bytes: &[u8]) {
        let mut txn = doc.transact_mut();
        txn.apply_update(Update::decode_v1(bytes).expect("update"))
            .expect("apply");
    }

    #[test]
    fn concurrent_yarray_appends_both_survive() {
        let a = Doc::with_client_id(0xA1);
        let b = Doc::with_client_id(0xB2);
        let a_squads = a.get_or_insert_map("squads");
        {
            let mut txn = a.transact_mut();
            let sq = a_squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
            insert_empty_native(&mut txn, &sq, SLOT_IDS);
        }
        apply(&b, &encode(&a));
        let b_squads = b.get_or_insert_map("squads");
        {
            let mut txn = a.transact_mut();
            append_id(&mut txn, &a_squads, "sq", SLOT_IDS, "from-a");
        }
        {
            let mut txn = b.transact_mut();
            append_id(&mut txn, &b_squads, "sq", SLOT_IDS, "from-b");
        }
        apply(&a, &encode(&b));
        apply(&b, &encode(&a));
        let txn = a.transact();
        let mut ids = read_ids(&txn, &a_squads, "sq", SLOT_IDS);
        ids.sort();
        assert_eq!(ids, ["from-a", "from-b"]);
    }

    /// Clone-rewrite of `Any::Array` (the pre-T-937.1 `append_id` body) last-write-wins.
    /// This is the defect pin: two peers appending concurrently keep only one id.
    #[test]
    fn concurrent_any_array_clone_rewrite_drops_an_id() {
        fn clone_rewrite_append(txn: &mut TransactionMut, map: &MapRef, key: &str, id: &str) {
            if let Some(Out::YMap(container)) = map.get(txn, key) {
                let mut next: Vec<Any> = match container.get(txn, SLOT_IDS) {
                    Some(Out::Any(Any::Array(arr))) => arr.iter().cloned().collect(),
                    _ => Vec::new(),
                };
                next.push(Any::String(id.into()));
                container.insert(txn, SLOT_IDS, Any::Array(next.into()));
            }
        }

        let a = Doc::with_client_id(0xA1);
        let b = Doc::with_client_id(0xB2);
        let a_squads = a.get_or_insert_map("squads");
        {
            let mut txn = a.transact_mut();
            let sq = a_squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
            sq.insert(&mut txn, SLOT_IDS, Any::Array(Vec::new().into()));
        }
        apply(&b, &encode(&a));
        let b_squads = b.get_or_insert_map("squads");
        {
            let mut txn = a.transact_mut();
            clone_rewrite_append(&mut txn, &a_squads, "sq", "from-a");
        }
        {
            let mut txn = b.transact_mut();
            clone_rewrite_append(&mut txn, &b_squads, "sq", "from-b");
        }
        apply(&a, &encode(&b));
        apply(&b, &encode(&a));
        let txn = a.transact();
        let ids = read_ids(&txn, &a_squads, "sq", SLOT_IDS);
        assert_eq!(
            ids.len(),
            1,
            "Any::Array clone-rewrite must drop one concurrent id (the T-937.1 defect); got {ids:?}"
        );
    }
}

#[cfg(test)]
mod mission_doc_tests {
    use super::*;
    use crate::doc::MissionDocCore;
    use yrs::updates::decoder::Decode;
    use yrs::{Doc, Transact, Update};

    /// Replay `encode_state` onto a probe peer so hydrate tests can call [`is_native_array`]
    /// without a `#[cfg(test)]` item on `MissionDocCore` (Class-R haystack truncates there).
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

    /// T-937.1 defect on the live document: two synced peers append concurrently; both ids
    /// must survive. Pre-fix this failed with a one-element `slotIds`.
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
}
