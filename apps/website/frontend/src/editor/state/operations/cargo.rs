//! Role: cargo.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use crate::editor::state::history as mission_history;
use std::collections::HashMap;

#[allow(unused_imports)]
use super::{batch::confirm_bulk_n_step, context::*, entity::*};

/// Read a slot's embedded `loadout` JSON (Arsenal picks) from `slots_json`. `None` when unset.
pub fn read_loadout(id: &str) -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::cargo::read_loadout(core, id)
    })
}

/// Install the character → default-cargo map (from the `/registry/compat` fetch).
pub fn set_cargo_defaults(
    map: HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>,
) {
    website_map_engine::data::store::operations::cargo::set_cargo_defaults(map);
}

/// Arsenal-open seed (pre-.15.2 slots): own ctx scope + history tail. Returns the seeded loadout JSON so the caller can render it without a re-read.
pub fn seed_slot_cargo(id: &str) -> Option<String> {
    let seeded = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::cargo::seed_slot_cargo_from_defaults(core, id)
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
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        website_map_engine::data::store::operations::cargo::buffer_loadouts_from_selection(
            core, sel,
        )
    })
}

/// What is in the buffer right now (for the panel's label and its receipt).
pub fn loadout_buffer() -> Vec<crate::editor::arsenal::BufferedLoadout> {
    website_map_engine::data::store::operations::cargo::loadout_buffer()
}

/// How many loadouts are buffered — the affordance the Apply button is enabled on.
pub fn loadout_buffer_len() -> usize {
    website_map_engine::data::store::operations::cargo::loadout_buffer_len()
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
    let seed = website_map_engine::data::store::operations::cargo::next_apply_seed();
    let writes = crate::editor::arsenal::plan_apply(&targets, &buffer, seed, items, feed)?;
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
        website_map_engine::data::store::operations::cargo::commit_loadout_writes(core, writes)
    });
    if commits > 0 {
        mission_history::after_local_edit();
    }
    commits
}
