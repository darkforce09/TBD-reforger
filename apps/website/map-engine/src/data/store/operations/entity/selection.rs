//! Role: selection.
//! Position: `doc/operations/entity` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;

/// Terrain key of using the supplied domain data.
pub fn terrain_key_of(core: &MissionDocCore) -> String {
    serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        .ok()
        .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Terrain bounds of using the supplied domain data.
pub fn terrain_bounds_of(core: &MissionDocCore) -> [f64; 4] {
    crate::data::scenario::compile::terrain_bounds(&terrain_key_of(core))
}

/// Selected slot ids using the supplied domain data.
pub fn selected_slot_ids(core: &MissionDocCore, sel: &[String]) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    sel.iter()
        .filter(|id| map.contains_key(id.as_str()))
        .cloned()
        .collect()
}

/// Read whether the current selection is ALL-hidden (used to decide the toggle direction and to drive a menu row's checked state). Returns `None` when the selection resolves to no slots (nothing to toggle). `Some(true)` ⇒ every selected slot is hidden (so a toggle SHOWS); `Some(false)` ⇒ at least one is visible (a toggle HIDES, matching Eden's "any visible → hide all" bias).
pub fn selection_all_hidden(core: &MissionDocCore, ids: &[String]) -> Option<bool> {
    if ids.is_empty() {
        return None;
    }
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let map = root.as_object()?;
    Some(ids.iter().all(|id| {
        map.get(id)
            .and_then(|s| s.get("editorHidden"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }))
}

/// `#[allow(dead_code)]`: the consuming dock render (an eye-off glyph on the slot row) is outside `owns` (`eden_tree`); this accessor is the shippable-visible datum it will read once wired.
#[allow(dead_code)]
#[must_use]
pub fn slot_hidden_rows(core: &MissionDocCore) -> Vec<(String, bool)> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<(String, bool)> = map
        .iter()
        .map(|(id, v)| {
            let hidden = v
                .get("editorHidden")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            (id.clone(), hidden)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// Apply set_selection_hidden to explicit document state.
pub fn set_selection_hidden(core: &MissionDocCore, hidden: bool, sel: Vec<String>) -> bool {
    let ids = selected_slot_ids(core, &sel);
    if ids.is_empty() {
        return false;
    }
    core.set_slots_editor_hidden(&ids, hidden);
    true
}

/// Apply toggle_hidden to explicit document state.
pub fn toggle_hidden(core: &MissionDocCore, sel: Vec<String>) -> Option<bool> {
    let ids = selected_slot_ids(core, &sel);

    selection_all_hidden(core, &ids).map(|all_hidden| !all_hidden)
}

/// Document operation over explicit authored state.
pub fn selection_centroid(core: &MissionDocCore, sel: &[String]) -> Option<(f64, f64)> {
    let soa = core.materialize();
    let mut sx = 0.0f64;
    let mut sy = 0.0f64;
    let mut n = 0.0f64;
    for id in sel {
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            sx += f64::from(soa.xs[row]);
            sy += f64::from(soa.ys[row]);
            n += 1.0;
        }
    }
    if n == 0.0 {
        return None;
    }
    Some((sx / n, sy / n))
}
