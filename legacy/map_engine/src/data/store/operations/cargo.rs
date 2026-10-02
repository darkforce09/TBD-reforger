//! Role: cargo.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::cargo_rules::{CargoRow, seed_cargo};
use super::entity::selected_slot_ids;
use crate::data::store::MissionDocCore;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

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

/// The odd increment a Weyl sequence advances [`APPLY_SEED`] by — the splitmix64 gamma. Advancing
/// before every Apply is what makes a second Apply of one buffer re-roll its randomised picks
/// instead of repeating the first Apply's.
const APPLY_SEED_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

thread_local! {
    /// Character asset id → the cargo rows a freshly placed one starts with, as the registry's
    /// compatibility feed authors them. Empty until the feed is installed, which is why seeding
    /// before then is a quiet no-op rather than a refusal.
    static CARGO_DEFAULTS: RefCell<HashMap<String, Vec<CargoRow>>> =
        RefCell::new(HashMap::new());

    /// What the last Copy lifted off the selection, held until the next Copy replaces it. It holds
    /// loadout BYTES rather than source ids, so an Apply can never silently follow a source that
    /// has been edited or deleted since it was copied.
    static LOADOUT_BUFFER: RefCell<Vec<BufferedLoadout>> = const { RefCell::new(Vec::new()) };

    /// The splitmix64 state every Apply draws its randomised picks from.
    static APPLY_SEED: Cell<u64> = const { Cell::new(0x2545_F491_4F6C_DD1D) };
}

/// Install the character → default-cargo map the registry's compatibility feed carries.
pub fn set_cargo_defaults(defaults: HashMap<String, Vec<CargoRow>>) {
    CARGO_DEFAULTS.with(|c| *c.borrow_mut() = defaults);
}

/// The cargo a freshly placed `asset_id` starts with, or `None` when no default is installed for
/// it.
#[must_use]
pub fn cargo_defaults_for(asset_id: &str) -> Option<Vec<CargoRow>> {
    CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned())
}

/// Seed one slot's cargo from the installed default for `asset_id`, inside a document borrow the
/// caller already holds — so the caller owns the history tail. Seeds only where a default exists
/// and the loadout carries no `cargo` key of its own.
pub fn seed_cargo_for_asset(
    core: &MissionDocCore,
    id: &str,
    asset_id: &str,
    loadout: Option<&str>,
) -> bool {
    seed_cargo_in_core(core, id, loadout, cargo_defaults_for(asset_id))
}

/// Seed one slot's cargo from the installed default for the asset the slot already names, and
/// return the seeded loadout JSON so the caller can render it without re-reading the document.
pub fn seed_slot_cargo_from_defaults(core: &MissionDocCore, id: &str) -> Option<String> {
    seed_slot_cargo(core, id, cargo_defaults_for)
}

/// Lift the loadout of every selected slot into the buffer, and return how many were lifted. A
/// Copy that finds nothing leaves the previous buffer standing: an errant click on empty space
/// must not destroy what the operator just copied.
pub fn buffer_loadouts_from_selection(core: &MissionDocCore, sel: Vec<String>) -> usize {
    let buffered = copy_loadouts_from_selection(core, sel);
    let count = buffered.len();
    if count > 0 {
        LOADOUT_BUFFER.with(|b| *b.borrow_mut() = buffered);
    }
    count
}

/// What is buffered right now — the Apply's source, and the panel's receipt.
#[must_use]
pub fn loadout_buffer() -> Vec<BufferedLoadout> {
    LOADOUT_BUFFER.with(|b| b.borrow().clone())
}

/// How many loadouts are buffered — the affordance the Apply control is enabled on.
#[must_use]
pub fn loadout_buffer_len() -> usize {
    LOADOUT_BUFFER.with(|b| b.borrow().len())
}

/// Draw the seed for one Apply, advancing the sequence so the next Apply draws a different one.
pub fn next_apply_seed() -> u64 {
    APPLY_SEED.with(|s| {
        let now = s.get();
        s.set(now.wrapping_add(APPLY_SEED_GAMMA));
        now
    })
}

#[cfg(test)]
#[path = "tests/cargo.rs"]
mod tests;
