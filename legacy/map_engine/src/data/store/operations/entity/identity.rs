//! Role: identity.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::layer_rows;

/// Mint an unused `layer-{n}` id, proven unique against the doc's live layer set.
pub fn mint_layer_id(core: &MissionDocCore, next_id: &std::cell::Cell<u32>) -> String {
    let existing: std::collections::HashSet<String> =
        layer_rows(core).into_iter().map(|l| l.id).collect();
    loop {
        let id = format!("layer-{}", next_id.get());
        next_id.set(next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// Auto-name a new layer "New Layer N" where N is the smallest positive integer not already used by an existing "New Layer …" name (so creating three in a row reads 1/2/3, and a delete-then-create reuses the gap). Names need not be unique in the doc; this is only a friendly default.
pub fn mint_layer_name(core: &MissionDocCore) -> String {
    let used: std::collections::HashSet<u32> = layer_rows(core)
        .iter()
        .filter_map(|l| {
            l.name
                .strip_prefix("New Layer ")?
                .trim()
                .parse::<u32>()
                .ok()
        })
        .collect();
    let mut n = 1u32;
    while used.contains(&n) {
        n += 1;
    }
    format!("New Layer {n}")
}

/// Every slot id the document actually holds — the slot half of both minters' uniqueness universe, read off the EXACT [`MissionDocCore::slots_json`] row map.
pub fn live_slot_ids(core: &MissionDocCore) -> std::collections::HashSet<String> {
    serde_json::from_str::<serde_json::Value>(&core.slots_json())
        .ok()
        .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

/// Slot attrs exists using the supplied domain data.
pub fn slot_attrs_exists(core: &MissionDocCore, id: &str) -> bool {
    core.slot_exists(id)
}

/// Document operation over explicit authored state.
pub fn mint_id(core: &MissionDocCore, next_id: &std::cell::Cell<u32>) -> String {
    let existing = live_slot_ids(core);
    loop {
        let id = format!("n{}", next_id.get());
        next_id.set(next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// Document operation over explicit authored state.
pub fn mint_ids(
    core: &MissionDocCore,
    next_id: &std::cell::Cell<u32>,
    count: usize,
) -> Vec<String> {
    let mut existing = live_slot_ids(core);
    if let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) {
        for key in ["vehiclesById", "entitiesById", "commentsById"] {
            if let Some(obj) = small.get(key).and_then(|v| v.as_object()) {
                existing.extend(obj.keys().cloned());
            }
        }
    }
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let id = format!("n{}", next_id.get());
        next_id.set(next_id.get().saturating_add(1));
        if existing.insert(id.clone()) {
            out.push(id);
        }
    }
    out
}
