//! Role: cargo.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use crate::editor::state::history as mission_history;
use map_engine_core::doc::MissionDocCore;
use std::cell::RefCell;
use std::collections::HashMap;

#[allow(unused_imports)]
use super::{attrs::*, compositions::*, context::*, entity::*, transform::*};

/// Read a slot's embedded `loadout` JSON (Arsenal picks) from `slots_json`. `None` when unset.
pub fn read_loadout(id: &str) -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_mission_core::doc::operations::cargo::read_loadout(core, id)
    })
}

thread_local! {

    static CARGO_DEFAULTS: RefCell<HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>> =
        RefCell::new(HashMap::new());
}

/// Install the character → default-cargo map (from the `/registry/compat` fetch).
pub fn set_cargo_defaults(
    map: HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>,
) {
    CARGO_DEFAULTS.with(|c| *c.borrow_mut() = map);
}

/// Seed one slot's cargo inside an already-open doc borrow (shared by the place / apply-kit hooks — the caller owns the history tail). Seeds only when the character has defaults and the loadout carries no `cargo` key.
pub(super) fn seed_cargo_in_core(
    core: &MissionDocCore,
    id: &str,
    asset_id: &str,
    loadout: Option<&str>,
) -> bool {
    let defaults = CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned());
    website_mission_core::doc::operations::cargo::seed_cargo_in_core(core, id, loadout, defaults)
}

/// Arsenal-open seed (pre-.15.2 slots): own ctx scope + history tail. Returns the seeded loadout JSON so the caller can render it without a re-read.
pub fn seed_slot_cargo(id: &str) -> Option<String> {
    let seeded = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_mission_core::doc::operations::cargo::seed_slot_cargo(core, id, |asset_id| {
            CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned())
        })
    });
    if seeded.is_some() {
        mission_history::after_local_edit();
    }
    seeded
}

/// Set/clear a slot's `loadout` (Arsenal commit) + the shared tail (one undo step). `None`/empty clears the key.
pub fn set_loadout(id: &str, loadout_json: Option<String>) -> bool {
    crate::editor::arsenal::commit_one_write(
        || {
            OPS_CTX.with(|c| {
                let guard = c.borrow();
                let Some(ctx) = guard.as_ref() else {
                    return false;
                };
                let d = ctx.doc.borrow();
                let Some(core) = d.as_ref() else {
                    return false;
                };
                core.update_slot_loadout(id, loadout_json)
            })
        },
        || {
            mission_history::after_local_edit();
        },
    )
}

thread_local! {

    static LOADOUT_BUFFER: RefCell<Vec<crate::editor::arsenal::BufferedLoadout>> =
        const { RefCell::new(Vec::new()) };

    static APPLY_SEED: std::cell::Cell<u64> = const { std::cell::Cell::new(0x2545_F491_4F6C_DD1D) };
}

fn next_apply_seed() -> u64 {
    APPLY_SEED.with(|s| {
        let now = s.get();
        s.set(now.wrapping_add(0x9E37_79B9_7F4A_7C15));
        now
    })
}

fn selection_slot_targets() -> Vec<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        selected_slot_ids(core, &sel)
    })
}

/// **Copy** (3DEN-LOAD-001) — buffer the loadout of EVERY selected entity, not just one. Returns how many were buffered.
pub fn copy_loadouts_from_selection() -> usize {
    let buffered = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::cargo::copy_loadouts_from_selection(core, sel)
    });
    let n = buffered.len();
    if n > 0 {
        LOADOUT_BUFFER.with(|b| *b.borrow_mut() = buffered);
    }
    n
}

/// What is in the buffer right now (for the panel's label and its receipt).
pub fn loadout_buffer() -> Vec<crate::editor::arsenal::BufferedLoadout> {
    LOADOUT_BUFFER.with(|b| b.borrow().clone())
}

/// How many loadouts are buffered — the affordance the Apply button is enabled on.
pub fn loadout_buffer_len() -> usize {
    LOADOUT_BUFFER.with(|b| b.borrow().len())
}

/// Apply loadout buffer to selection using the supplied domain data.
pub fn apply_loadout_buffer_to_selection(
    items: &[crate::v2::core::api::dto::RegistryItem],
    feed: &crate::editor::arsenal::arsenal_rules::CompatFeed,
) -> Result<(usize, usize), Vec<crate::editor::arsenal::arsenal_rules::RowError>> {
    let buffer = loadout_buffer();
    let targets = selection_slot_targets();
    if buffer.is_empty() || targets.is_empty() {
        return Ok((0, 0));
    }
    if !confirm_bulk_n_step(targets.len(), "overwrite the loadout of") {
        return Ok((0, 0));
    }
    let writes =
        crate::editor::arsenal::plan_apply(&targets, &buffer, next_apply_seed(), items, feed)?;
    Ok((writes.len(), commit_loadout_writes(&writes)))
}

/// **Remove Everything** (3DEN-LOAD-010) — strip every selected entity's loadout. Returns `(planned, committed)`, for the same reason Apply does.
pub fn remove_all_loadouts_from_selection() -> (usize, usize) {
    let targets = selection_slot_targets();
    if targets.is_empty() {
        return (0, 0);
    }
    if !confirm_bulk_n_step(targets.len(), "remove every item from") {
        return (0, 0);
    }
    let writes = crate::editor::arsenal::plan_remove(&targets);
    (writes.len(), commit_loadout_writes(&writes))
}

fn commit_loadout_writes(writes: &[crate::editor::arsenal::LoadoutWrite]) -> usize {
    if writes.is_empty() {
        return 0;
    }
    let commits = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        website_mission_core::doc::operations::cargo::commit_loadout_writes(core, writes)
    });
    if commits > 0 {
        mission_history::after_local_edit();
    }
    commits
}
