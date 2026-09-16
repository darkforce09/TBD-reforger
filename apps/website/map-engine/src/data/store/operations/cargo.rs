//! Role: cargo.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::cargo_rules::{CargoRow, seed_cargo};
use super::entity::selected_slot_ids;
use crate::data::store::MissionDocCore;

/// **A buffer, not an inheritance hierarchy — and that is the whole design.**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferedLoadout {
    /// Source id.
    pub source_id: String,

    /// The source's `SlotLoadoutV2` JSON, verbatim. `None` when the source carried no `loadout` key at all — a **bare** entity, which is a legitimate thing to buffer and to apply (it is how you say "make these look like that empty one"), and which is not the same value as [`stripped_loadout`]; see there for why the two differ.
    pub loadout_json: Option<String>,
}

/// Domain representation of loadout write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadoutWrite {
    /// Target id.
    pub target_id: String,

    /// Which buffered entity this loadout was drawn from; `None` for Remove Everything, which has no source.
    pub source_id: Option<String>,

    /// Loadout json.
    pub loadout_json: Option<String>,
}

/// Push a plan into the document, and **return how many writes actually reached it**.
pub fn commit_writes(
    writes: &[LoadoutWrite],
    mut commit: impl FnMut(&str, Option<String>) -> bool,
) -> usize {
    let mut done = 0usize;
    for w in writes {
        if commit(&w.target_id, w.loadout_json.clone()) {
            done += 1;
        }
    }
    done
}

/// Apply read_loadout to explicit document state.
pub fn read_loadout(core: &MissionDocCore, id: &str) -> Option<String> {
    let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
    let lo = map.get(id)?.get("loadout")?;
    if lo.is_null() {
        return None;
    }
    Some(lo.to_string())
}

/// Apply copy_loadouts_from_selection to explicit document state.
pub fn copy_loadouts_from_selection(
    core: &MissionDocCore,
    sel: Vec<String>,
) -> Vec<BufferedLoadout> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    selected_slot_ids(core, &sel)
        .into_iter()
        .map(|id| {
            let loadout_json = map
                .get(&id)
                .and_then(|s| s.get("loadout"))
                .filter(|l| !l.is_null())
                .map(ToString::to_string);
            BufferedLoadout {
                source_id: id,
                loadout_json,
            }
        })
        .collect()
}

/// Apply commit_loadout_writes to explicit document state.
pub fn commit_loadout_writes(core: &MissionDocCore, writes: &[LoadoutWrite]) -> usize {
    commit_writes(writes, |id, json| core.update_slot_loadout(id, json))
}

/// Document operation over explicit authored state.
pub fn seed_cargo_in_core(
    core: &MissionDocCore,
    id: &str,
    loadout: Option<&str>,
    defaults: Option<Vec<CargoRow>>,
) -> bool {
    let Some(defaults) = defaults else {
        return false;
    };
    match seed_cargo(loadout, &defaults) {
        Some(json) => core.update_slot_loadout(id, Some(json)),
        None => false,
    }
}

/// Apply seed_slot_cargo to explicit document state.
pub fn seed_slot_cargo(
    core: &MissionDocCore,
    id: &str,
    defaults_for: impl FnOnce(&str) -> Option<Vec<CargoRow>>,
) -> Option<String> {
    let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
    let slot = map.get(id)?;
    let asset_id = slot.get("assetId")?.as_str().filter(|s| !s.is_empty())?;
    let loadout = slot
        .get("loadout")
        .filter(|l| !l.is_null())
        .map(|l| l.to_string());
    let defaults = defaults_for(asset_id)?;
    let json = seed_cargo(loadout.as_deref(), &defaults)?;

    core.update_slot_loadout(id, Some(json.clone()))
        .then_some(json)
}
