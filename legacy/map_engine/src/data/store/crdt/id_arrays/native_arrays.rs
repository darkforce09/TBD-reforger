//! Role: native arrays.
//! Position: `doc/crdt/id_arrays` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Array;
use super::ArrayPrelim;
use super::HashSet;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::TransactionMut;
use yrs::Map;

/// Canonical slot ids value.
pub const SLOT_IDS: &str = "slotIds";

/// Canonical entity ids value.
pub const ENTITY_IDS: &str = "entityIds";

/// Is native field using the supplied domain data.
pub(super) fn is_native_field(field: &str) -> bool {
    field == SLOT_IDS || field == ENTITY_IDS
}

/// Any id string using the supplied domain data.
pub(super) fn any_id_string(a: &Any) -> Option<String> {
    match a {
        Any::String(s) => Some(s.to_string()),
        _ => None,
    }
}

/// Out id string using the supplied domain data.
pub(super) fn out_id_string(out: &Out) -> Option<String> {
    match out {
        Out::Any(a) => any_id_string(a),
        _ => None,
    }
}

/// Ids from any array using the supplied domain data.
pub(super) fn ids_from_any_array(arr: &[Any]) -> Vec<Any> {
    arr.to_vec()
}

/// Ids from yarray using the supplied domain data.
pub(super) fn ids_from_yarray<T: ReadTxn>(txn: &T, arr: &yrs::ArrayRef) -> Vec<Any> {
    arr.iter(txn)
        .filter_map(|out| out_id_string(&out).map(|s| Any::String(s.into())))
        .collect()
}

/// Strings from anys using the supplied domain data.
pub(super) fn strings_from_anys(anys: &[Any]) -> Vec<String> {
    anys.iter().filter_map(any_id_string).collect()
}

/// Read `map[key].field` (string ids) as owned `Any` values. Empty when the container or field is missing. Accepts native `YArray` and legacy `Any::Array` identically and in order.
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
#[allow(dead_code)]
pub fn read_ids<T: ReadTxn>(txn: &T, map: &MapRef, key: &str, field: &str) -> Vec<String> {
    strings_from_anys(&read_id_array(txn, map, key, field))
}

/// Ordered string ids for `container.field` (both representations).
pub fn read_field_ids<T: ReadTxn>(txn: &T, container: &MapRef, field: &str) -> Vec<String> {
    strings_from_anys(&read_field(txn, container, field))
}

/// True when `map[key].field` is a live `YArray` (not a legacy `Any::Array`).
#[allow(dead_code)]
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

/// Ensure native using the supplied domain data.
pub(super) fn ensure_native(txn: &mut TransactionMut, container: &MapRef, field: &str) {
    match container.get(txn, field) {
        Some(Out::YArray(_)) => {}
        Some(Out::Any(Any::Array(arr))) => {
            let ids = strings_from_anys(&ids_from_any_array(&arr));
            replace_native(txn, container, field, &ids);
        }
        _ => insert_empty_native(txn, container, field),
    }
}

/// Filter a cloned `Any::Array` (opaque lists: `vehicleIds`, `squadIds`). Native fields must use [`retain_in`] so the live `YArray` is edited in place.
pub fn retain_ids(arr: &[Any], remove: &HashSet<&str>) -> Vec<Any> {
    arr.iter()
        .filter(|a| !matches!(a, Any::String(s) if remove.contains(s.as_ref())))
        .cloned()
        .collect()
}

/// Drop every id in `remove` from `container.field`. Native fields delete `YArray` entries; opaque fields clone-rewrite `Any::Array`.
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

/// Append `id` to `map[key].field` if that container map exists. Native fields `push_back` on a `YArray` (CRDT merge); opaque fields clone-rewrite `Any::Array`. Dedup: an id already present is not appended again (a slot belongs to a squad once).
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

/// Move `id` to `to_index` within a native id list (clamped). No-op when the id is absent or the field is not a `YArray`.
#[allow(dead_code)]
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

/// Hydrate-time migration: every `squad.slotIds` and `layer.entityIds` that is still a legacy `Any::Array` becomes a native `YArray` once, preserving order. Missing keys stay missing so a payload that omitted them does not grow a new `[]` on the wire.
pub fn migrate_legacy_id_lists(txn: &mut TransactionMut, squads: &MapRef, editor_layers: &MapRef) {
    migrate_map_field(txn, squads, SLOT_IDS);
    migrate_map_field(txn, editor_layers, ENTITY_IDS);
}

/// Migrate map field using the supplied domain data.
pub(super) fn migrate_map_field(txn: &mut TransactionMut, parent: &MapRef, field: &str) {
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
