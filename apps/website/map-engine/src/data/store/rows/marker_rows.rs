//! Role: marker rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MapRef;
use super::Out;
use super::PENDING_BRIEFING_MARKERS;
use super::ReadTxn;
use super::read_any_map;

/// Briefing markers using the supplied domain data.
pub(super) fn briefing_markers(briefing: &HashMap<String, Any>) -> Vec<Any> {
    match briefing.get("markers") {
        Some(Any::Array(arr)) => arr.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

/// Pending briefing markers map using the supplied domain data.
pub(super) fn pending_briefing_markers_map<T: ReadTxn>(
    txn: &T,
    meta: &MapRef,
) -> HashMap<String, Vec<Any>> {
    let Some(Out::Any(Any::Map(m))) = meta.get(txn, PENDING_BRIEFING_MARKERS) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for (faction_id, val) in m.iter() {
        let Any::Array(arr) = val else { continue };
        out.insert(faction_id.clone(), arr.iter().cloned().collect());
    }
    out
}

/// Write pending briefing markers map using the supplied domain data.
pub(super) fn write_pending_briefing_markers_map(
    txn: &mut yrs::TransactionMut<'_>,
    meta: &MapRef,
    pending: HashMap<String, Vec<Any>>,
) {
    if pending.is_empty() {
        meta.remove(txn, PENDING_BRIEFING_MARKERS);
        return;
    }
    let mut map = HashMap::new();
    for (faction_id, markers) in pending {
        map.insert(faction_id, Any::Array(markers.into()));
    }
    meta.insert(txn, PENDING_BRIEFING_MARKERS, Any::Map(Arc::new(map)));
}

/// Upsert pending briefing marker using the supplied domain data.
pub(super) fn upsert_pending_briefing_marker(
    txn: &mut yrs::TransactionMut<'_>,
    meta: &MapRef,
    faction_id: &str,
    marker_id: &str,
    row: Any,
) {
    let mut pending = pending_briefing_markers_map(txn, meta);
    let markers = pending.entry(faction_id.to_string()).or_default();
    match markers
        .iter()
        .position(|m| marker_row_id(m) == Some(marker_id))
    {
        Some(i) => markers[i] = row,
        None => markers.push(row),
    }
    write_pending_briefing_markers_map(txn, meta, pending);
}

/// Remove pending briefing marker using the supplied domain data.
pub(super) fn remove_pending_briefing_marker(
    txn: &mut yrs::TransactionMut<'_>,
    meta: &MapRef,
    faction_id: &str,
    marker_id: &str,
) {
    let mut pending = pending_briefing_markers_map(txn, meta);
    let Some(markers) = pending.get_mut(faction_id) else {
        return;
    };
    let before = markers.len();
    markers.retain(|m| marker_row_id(m) != Some(marker_id));
    if markers.len() == before {
        return;
    }
    if markers.is_empty() {
        pending.remove(faction_id);
    }
    write_pending_briefing_markers_map(txn, meta, pending);
}

/// Promote pending briefing markers using the supplied domain data.
pub(super) fn promote_pending_briefing_markers(
    txn: &mut yrs::TransactionMut<'_>,
    meta: &MapRef,
    faction: &MapRef,
    faction_id: &str,
) {
    let mut pending = pending_briefing_markers_map(txn, meta);
    let Some(parked) = pending.remove(faction_id) else {
        return;
    };
    if parked.is_empty() {
        write_pending_briefing_markers_map(txn, meta, pending);
        return;
    }
    let mut briefing = read_any_map(txn, faction, "briefing");
    let mut markers = briefing_markers(&briefing);
    for row in parked {
        let Some(id) = marker_row_id(&row).map(str::to_string) else {
            markers.push(row);
            continue;
        };
        match markers
            .iter()
            .position(|m| marker_row_id(m) == Some(id.as_str()))
        {
            Some(i) => markers[i] = row,
            None => markers.push(row),
        }
    }
    briefing.insert("markers".to_string(), Any::Array(markers.into()));
    faction.insert(txn, "briefing", Any::Map(Arc::new(briefing)));
    write_pending_briefing_markers_map(txn, meta, pending);
}

/// Marker row id using the supplied domain data.
pub(super) fn marker_row_id(row: &Any) -> Option<&str> {
    let Any::Map(fields) = row else { return None };
    match fields.get("id") {
        Some(Any::String(s)) => Some(s.as_ref()),
        _ => None,
    }
}

/// Marker any using the supplied domain data.
pub(super) fn marker_any(id: &str, x: f64, z: f64, icon: &str, label: &str) -> Any {
    Any::Map(Arc::new(HashMap::from([
        ("id".to_string(), Any::String(id.into())),
        ("x".to_string(), Any::Number(x)),
        ("z".to_string(), Any::Number(z)),
        ("icon".to_string(), Any::String(icon.into())),
        ("label".to_string(), Any::String(label.into())),
    ])))
}
