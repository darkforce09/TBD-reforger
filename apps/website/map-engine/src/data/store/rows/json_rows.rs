//! Role: json rows.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::HashSet;
use super::Map;
use super::MapPrelim;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::STANCE_CROUCH;
use super::STANCE_PRONE;
use super::STANCE_STAND;
use super::TransactionMut;
use yrs::types::ToJson;

/// Any map str using the supplied domain data.
pub(super) fn any_map_str(m: &HashMap<String, Any>, key: &str) -> Option<String> {
    match m.get(key) {
        Some(Any::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// Json str using the supplied domain data.
pub(super) fn json_str(
    m: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    m.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Json num using the supplied domain data.
pub(super) fn json_num(m: &serde_json::Map<String, serde_json::Value>, key: &str) -> f64 {
    m.get(key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

/// Copy row fields except using the supplied domain data.
pub(super) fn copy_row_fields_except(
    txn: &mut TransactionMut,
    entity: &MapRef,
    row: &serde_json::Map<String, serde_json::Value>,
    skip: &[&str],
) {
    for (k, v) in row {
        if skip.contains(&k.as_str()) {
            continue;
        }
        entity.insert(txn, k.as_str(), value_to_any(v));
    }
}

/// Read env map using the supplied domain data.
pub(super) fn read_env_map<T: ReadTxn>(txn: &T, meta: &MapRef) -> HashMap<String, Any> {
    match meta.get(txn, "environment") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}

/// Is known editor payload top level using the supplied domain data.
pub(super) fn is_known_editor_payload_top_level(key: &str) -> bool {
    matches!(
        key,
        "schemaVersion"
            | "map"
            | "environment"
            | "title"
            | "loadouts"
            | "objectives"
            | "vehicles"
            | "entities"
            | "zones"
            | "compositions"
            | "triggers"
            | "comments"
            | "connections"
            | "markers"
            | "editor"
            | "orbat"
            | "payloadExtras"
    )
}

/// Json str to any using the supplied domain data.
pub(super) fn json_str_to_any(s: &str) -> Any {
    serde_json::from_str::<serde_json::Value>(s).map_or(Any::Null, |v| value_to_any(&v))
}

/// Value to any using the supplied domain data.
pub(super) fn value_to_any(v: &serde_json::Value) -> Any {
    match v {
        serde_json::Value::Null => Any::Null,
        serde_json::Value::Bool(b) => Any::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Any::Number(n.as_f64().unwrap_or(0.0)), Any::BigInt),
        serde_json::Value::String(s) => Any::String(s.as_str().into()),
        serde_json::Value::Array(arr) => {
            Any::Array(arr.iter().map(value_to_any).collect::<Vec<_>>().into())
        }
        serde_json::Value::Object(map) => {
            let m: HashMap<String, Any> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_any(v)))
                .collect();
            Any::Map(Arc::new(m))
        }
    }
}

/// Ordered rows using the supplied domain data.
pub(super) fn ordered_rows(
    txn: &impl ReadTxn,
    map: &MapRef,
    entity_order: &MapRef,
    order_key: &str,
) -> Vec<Any> {
    let by_id: HashMap<String, Any> = map
        .iter(txn)
        .map(|(id, out)| (id.to_string(), out.to_json(txn)))
        .collect();
    if by_id.is_empty() {
        return Vec::new();
    }
    let order: Option<Vec<String>> = match entity_order.get(txn, order_key) {
        Some(Out::Any(Any::Array(arr))) => Some(
            arr.iter()
                .filter_map(|v| match v {
                    Any::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    };
    let Some(order) = order else {
        return map.iter(txn).map(|(_, out)| out.to_json(txn)).collect();
    };

    let mut seen: HashSet<&str> = HashSet::new();
    let mut out: Vec<Any> = Vec::with_capacity(by_id.len());
    for id in &order {
        if let Some(row) = by_id.get(id.as_str())
            && seen.insert(id.as_str())
        {
            out.push(row.clone());
        }
    }
    for (id, row) in map.iter(txn) {
        if !seen.contains(id) {
            out.push(row.to_json(txn));
        }
    }
    out
}

/// Load rows ordered using the supplied domain data.
pub(super) fn load_rows_ordered(
    txn: &mut TransactionMut,
    map: &MapRef,
    rows: Option<&Any>,
    entity_order: &MapRef,
    order_key: &str,
) {
    let mut ids: Vec<Any> = Vec::new();
    if let Some(Any::Array(arr)) = rows {
        for row in arr.iter() {
            if let Any::Map(fields) = row
                && let Some(Any::String(id)) = fields.get("id")
            {
                ids.push(Any::String(id.clone()));
            }
            load_row(txn, map, row);
        }
    }
    if !ids.is_empty() {
        entity_order.insert(txn, order_key, Any::Array(ids.into()));
    }
}

/// Load row using the supplied domain data.
pub(super) fn load_row(txn: &mut TransactionMut, map: &MapRef, row: &Any) {
    let Any::Map(fields) = row else { return };
    let Some(Any::String(id)) = fields.get("id") else {
        return;
    };
    let id = id.as_ref();
    let entity = map.insert(&mut *txn, id, MapPrelim::from([("id", id)]));
    for (k, v) in fields.iter() {
        if k != "id" {
            entity.insert(&mut *txn, k.as_str(), v.clone());
        }
    }
}

/// Any to f64 using the supplied domain data.
pub(super) fn any_to_f64(a: &Any) -> f64 {
    match a {
        Any::Number(n) => *n,
        Any::BigInt(i) => *i as f64,
        Any::Bool(true) => 1.0,
        Any::Bool(false) => 0.0,
        _ => 0.0,
    }
}

/// Read any map using the supplied domain data.
pub(super) fn read_any_map<T: ReadTxn>(txn: &T, row: &MapRef, key: &str) -> HashMap<String, Any> {
    match row.get(txn, key) {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}

/// Canonical pending briefing markers value.
pub(super) const PENDING_BRIEFING_MARKERS: &str = "pendingBriefingMarkers";

/// Read str using the supplied domain data.
pub(super) fn read_str<T: ReadTxn>(txn: &T, slot: &MapRef, key: &str) -> Option<String> {
    match slot.get(txn, key) {
        Some(Out::Any(Any::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Read bool using the supplied domain data.
pub(super) fn read_bool<T: ReadTxn>(txn: &T, row: &MapRef, key: &str) -> bool {
    matches!(row.get(txn, key), Some(Out::Any(Any::Bool(true))))
}

/// Read stance using the supplied domain data.
pub(super) fn read_stance<T: ReadTxn>(txn: &T, slot: &MapRef) -> u8 {
    match read_str(txn, slot, "stance").as_deref() {
        Some("crouch") => STANCE_CROUCH,
        Some("prone") => STANCE_PRONE,
        _ => STANCE_STAND,
    }
}
