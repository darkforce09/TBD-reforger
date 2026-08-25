//! T-934.7 — loadout / cargo half of the old `state/operations.rs`: `read_loadout` /
//! `set_loadout`, cargo seeding and the T-699 loadout buffer verbs.
//! Split from `operations.rs`; the façade re-exports keep paths stable.

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
        let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
        let lo = map.get(id)?.get("loadout")?;
        if lo.is_null() {
            return None;
        }
        Some(lo.to_string())
    })
}

thread_local! {
    /// T-068.15.2 — per-character default cargo (registry `character_default_cargo`
    /// edges, aggregated). Filled by the editor's compat fetch; consumed by the
    /// seed hooks (place / apply-kit / Arsenal open).
    static CARGO_DEFAULTS: RefCell<HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>> =
        RefCell::new(HashMap::new());
}

/// Install the character → default-cargo map (from the `/registry/compat` fetch).
pub fn set_cargo_defaults(
    map: HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>,
) {
    CARGO_DEFAULTS.with(|c| *c.borrow_mut() = map);
}

/// Seed one slot's cargo inside an already-open doc borrow (shared by the place /
/// apply-kit hooks — the caller owns the history tail). Seeds only when the
/// character has defaults and the loadout carries no `cargo` key.
pub(super) fn seed_cargo_in_core(
    core: &MissionDocCore,
    id: &str,
    asset_id: &str,
    loadout: Option<&str>,
) -> bool {
    let defaults = CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned());
    let Some(defaults) = defaults else {
        return false;
    };
    match crate::editor::arsenal::arsenal_rules::seed_cargo(loadout, &defaults) {
        // T-779 — the SINK's answer, not a hardcoded `true`. `update_slot_loadout` returns `false`
        // for an id the document does not hold (T-770), and "a seed was computed" is a different
        // claim from "the document took it". Every current caller discards this bool, which is
        // precisely why it had to stop lying: the next reader would inherit a flag that was only
        // ever right by accident.
        Some(json) => core.update_slot_loadout(id, Some(json)),
        None => false,
    }
}

/// Arsenal-open seed (pre-.15.2 slots): own ctx scope + history tail. Returns the
/// seeded loadout JSON so the caller can render it without a re-read.
pub fn seed_slot_cargo(id: &str) -> Option<String> {
    let seeded = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
        let slot = map.get(id)?;
        let asset_id = slot.get("assetId")?.as_str().filter(|s| !s.is_empty())?;
        let loadout = slot
            .get("loadout")
            .filter(|l| !l.is_null())
            .map(|l| l.to_string());
        let defaults = CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned())?;
        let json =
            crate::editor::arsenal::arsenal_rules::seed_cargo(loadout.as_deref(), &defaults)?;
        // T-779 — `Some(json)` only if the DOCUMENT took the write. The `map.get(id)?` above makes
        // a refusal hard to reach today, but the tail below (`after_local_edit`) mints an undo step
        // and dirties the mission off this `Option` alone, so it must carry the sink's answer
        // rather than the fact that we got this far.
        core.update_slot_loadout(id, Some(json.clone()))
            .then_some(json)
    });
    if seeded.is_some() {
        mission_history::after_local_edit();
    }
    seeded
}

/// Set/clear a slot's `loadout` (Arsenal commit) + the shared tail (one undo step). `None`/empty
/// clears the key.
///
/// **Returns whether the DOCUMENT took the write** (T-779). Until this slice `did` was a hardcoded
/// `true` sitting directly under `core.update_slot_loadout(…);`, which threw away the `bool` T-770
/// had just added to the mutator for exactly this purpose. Two things were wrong with that:
///
/// 1. The tail fired whenever [`OPS_CTX`] and the doc merely EXISTED. A write against a stale or
///    deleted slot id — the Arsenal holds the id it opened with, and that entity can be deleted or
///    undone out from under the modal — still dirtied the mission and minted an undo step over a
///    document that had not changed. Ctrl+Z then had a step that restored nothing.
/// 2. Nothing built on this path could tell a real write from a no-op, which is the whole defect
///    T-770 was filed to close. `arsenal::commit_writes` (the multi-write sibling) counts the sink
///    and gets this right; the single-write path did not.
///
/// The `bool` is returned rather than swallowed because the operator has to be able to learn about
/// a refusal: `ArsenalTab`'s persistence line would otherwise render its green "no unsaved changes"
/// verdict over a pick that never landed. See `arsenal.rs`'s `persist` closure.
///
/// The gate itself lives in [`crate::editor::arsenal::commit_one_write`] and not in an `if` here, because
/// this module is `cfg(target_arch = "wasm32")` from line one and a native test cannot reach it —
/// the same reason T-770 put the batch loop in `arsenal::commit_writes`. There the refusal→no-tail
/// arithmetic is *driven* by `arsenal::tests::t779`; here it could only be read.
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

/* ═════ T-699 (3DEN-LOAD-001 / -002 / -010) — the loadout BUFFER: Copy · Apply · Remove Everything ═════ */

thread_local! {
    /// The loadout buffer. Deliberately a **snapshot of bytes**, not a list of source ids: T-687's
    /// loadout INHERITANCE was cancelled by the operator, and a buffer that stored ids would have to
    /// re-read the sources at Apply time, which is inheritance with extra steps. Copy takes the
    /// bytes; the sources can then be edited, or deleted, and the buffer neither knows nor cares.
    ///
    /// Separate from `CLIPBOARD` on purpose. That one holds whole slot dicts for Ctrl+C/Ctrl+V and
    /// a copied *entity* is a different thing from a copied *kit*; sharing one cell would mean
    /// Ctrl+C silently destroying a loadout buffer the author was in the middle of using.
    static LOADOUT_BUFFER: RefCell<Vec<crate::editor::arsenal::BufferedLoadout>> =
        const { RefCell::new(Vec::new()) };

    /// The Apply seed stream. Fixed start + a fixed step (see `next_apply_seed`), so the Nth Apply
    /// of a session always draws the Nth assignment: pressing Apply twice re-rolls, and a bug report
    /// that names a press replays exactly. No clock, no JS RNG — see `arsenal::buffer_draw` for the
    /// full argument.
    static APPLY_SEED: std::cell::Cell<u64> = const { std::cell::Cell::new(0x2545_F491_4F6C_DD1D) };
}

/// Advance the session's Apply seed and return the value this Apply draws with. The step is the
/// SplitMix64 gamma, so consecutive Applies land in unrelated parts of the stream rather than in
/// adjacent ones. Advances on every ATTEMPT, including a refused one — burning a seed costs nothing
/// and the alternative (advance only on success) makes "the third Apply" ambiguous in exactly the
/// bug reports the fixed stream exists to make replayable.
fn next_apply_seed() -> u64 {
    APPLY_SEED.with(|s| {
        let now = s.get();
        s.set(now.wrapping_add(0x9E37_79B9_7F4A_7C15));
        now
    })
}

/// The selection, filtered to slot ids, read in one doc borrow. Vehicles and objects are dropped:
/// `loadout` is a slot column, so counting a vehicle would report "applied to 5" over 4 documents.
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

/// **Copy** (3DEN-LOAD-001) — buffer the loadout of EVERY selected entity, not just one. Returns how
/// many were buffered.
///
/// An entity with no `loadout` key buffers as a `None` — a bare kit is a real thing to copy, and
/// dropping those would silently make a 5-entity Copy a 3-entity buffer. A Copy that resolves to
/// **nothing** (empty selection, or a selection of vehicles only) leaves the previous buffer intact
/// rather than clearing it: an accidental click must not destroy work the author is mid-way through
/// using, and "0 copied" is already said in the receipt.
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
                crate::editor::arsenal::BufferedLoadout {
                    source_id: id,
                    loadout_json,
                }
            })
            .collect()
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

/// **Apply** (3DEN-LOAD-002) — write the buffer onto the selection, drawing one buffered loadout per
/// entity at random. `Ok((planned, committed))` is how many writes the plan held and how many the
/// document actually took; `Err` is the refusal list, and a refusal writes **nothing at all**. Both
/// numbers are returned rather than just the second so the caller's receipt can say when they
/// disagree instead of quietly reporting the optimistic one.
///
/// The gate is `arsenal::plan_apply`, which is T-686's import gate over the whole buffer — the same
/// three rule passes, run before a die is rolled so the verdict cannot depend on the draw. Planning
/// is complete before the write borrow opens, so there is no path on which half a plan reaches the
/// document because the other half was refused.
///
/// ⚠️ **N ENTITIES IS N UNDO STEPS, and this is where that is true.** `update_slot_loadout` opens one
/// Yrs transaction per call and the store's `capture_timeout_millis = 0` makes every transaction its
/// own undo step, so the loop below costs one Ctrl+Z per entity. The core has no atomic multi-entity
/// loadout write — that is **T-732**, the same wall wave 111's T-645 hit and the same one
/// `commit_positions` documents a few hundred lines up — and `store.rs` is outside this slice's
/// `owns`. The single `after_local_edit()` below is the shared post-change tail (re-materialize,
/// dock rebuild, persist), NOT an undo boundary, and conflating the two is exactly the fake
/// atomicity this comment refuses. The author is told the real number: `arsenal::apply_receipt`
/// builds its line from the returned commit count.
pub fn apply_loadout_buffer_to_selection(
    items: &[crate::core::dto::RegistryItem],
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

/// **Remove Everything** (3DEN-LOAD-010) — strip every selected entity's loadout. Returns
/// `(planned, committed)`, for the same reason Apply does.
///
/// This is the ONE strip verb this slice builds. The nine per-category variants 3den also offers
/// (NVGs, vests, goggles, headgear, weapons, …) are marked `maybe` upstream and are deliberately out
/// of scope — see `arsenal::stripped_loadout`, which also explains why a strip writes an explicit
/// empty document rather than clearing the field (clearing it would let the cargo seed put the
/// author's magazines back).
///
/// Same N-steps-for-N-entities reality as Apply; see there and T-732.
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

/// The shared write path for both verbs: one doc borrow, one `update_slot_loadout` per planned
/// write, one shared post-change tail. Returns the number of writes the document actually took —
/// `arsenal::commit_writes` counts the sink, and both receipts are built from that count rather than
/// from the plan length, so a drop cannot be reported as a success.
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
        crate::editor::arsenal::commit_writes(writes, |id, json| core.update_slot_loadout(id, json))
    });
    if commits > 0 {
        mission_history::after_local_edit();
    }
    commits
}
