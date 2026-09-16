//! Role: layer rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::ENTITY_IDS;
use super::HashSet;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::TransactionMut;
use super::read_field_ids;
use super::retain_in;
use yrs::Map;

/// Layer flag using the supplied domain data.
pub(super) fn layer_flag<T: ReadTxn>(
    txn: &T,
    editor_layers: &MapRef,
    layer_id: &str,
    flag: &str,
) -> bool {
    matches!(
        editor_layers.get(txn, layer_id).and_then(|o| match o {
            Out::YMap(layer) => layer.get(txn, flag),
            _ => None,
        }),
        Some(Out::Any(Any::Bool(true)))
    )
}

/// Layer flag effective using the supplied domain data.
pub(super) fn layer_flag_effective<T: ReadTxn>(
    txn: &T,
    editor_layers: &MapRef,
    layer_id: &str,
    flag: &str,
) -> bool {
    let mut seen: HashSet<String> = HashSet::new();
    let mut cur = Some(layer_id.to_string());
    while let Some(c) = cur {
        if !seen.insert(c.clone()) {
            return false;
        }
        if layer_flag(txn, editor_layers, &c, flag) {
            return true;
        }
        cur = match editor_layers.get(txn, &c) {
            Some(Out::YMap(layer)) => match layer.get(txn, "parentId") {
                Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                _ => None,
            },
            _ => None,
        };
    }
    false
}

/// Slot first layer using the supplied domain data.
pub(super) fn slot_first_layer<T: ReadTxn>(
    txn: &T,
    editor_layers: &MapRef,
    slot_id: &str,
) -> Option<String> {
    for (layer_id, out) in editor_layers.iter(txn) {
        if let Out::YMap(layer) = out
            && read_field_ids(txn, &layer, ENTITY_IDS)
                .iter()
                .any(|s| s == slot_id)
        {
            return Some(layer_id.to_string());
        }
    }
    None
}

/// Slot is transform locked using the supplied domain data.
pub(super) fn slot_is_transform_locked<T: ReadTxn>(
    txn: &T,
    editor_layers: &MapRef,
    slot_id: &str,
) -> bool {
    match slot_first_layer(txn, editor_layers, slot_id) {
        Some(layer_id) => layer_flag_effective(txn, editor_layers, &layer_id, "locked"),
        None => false,
    }
}

/// Remove id from all layers using the supplied domain data.
pub(super) fn remove_id_from_all_layers(
    txn: &mut TransactionMut,
    editor_layers: &MapRef,
    id: &str,
) {
    let layer_ids: Vec<String> = editor_layers
        .iter(txn)
        .map(|(k, _)| k.to_string())
        .collect();
    for lid in &layer_ids {
        if let Some(Out::YMap(layer)) = editor_layers.get(txn, lid) {
            retain_in(txn, &layer, ENTITY_IDS, &HashSet::from([id]));
        }
    }
}
