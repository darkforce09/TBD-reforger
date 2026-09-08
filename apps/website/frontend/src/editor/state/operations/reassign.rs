//! T-939.2 — batch faction / squad reassignment for the Attributes modal.
//!
//! One entry point, [`reassign_slots`], which moves **every** id it is given into one destination
//! squad inside **one** undo group. It is the operation half of the modal's faction selector and
//! squad picker; the pure decision that picks the destination (and writes the refusals) is
//! [`crate::editor::panels::attributes_modal::plan_reassign`], which lives beside the modal because
//! it must be reachable from `cargo test` — this module is `wasm32`-only (its parent façade is), so
//! a decision buried in here could only ever be pinned by scraping source.
//!
//! **Two things this module is careful about.**
//!
//! *The source squad survives.* Moves go through
//! [`map_engine_core::doc::MissionDocCore::move_slot_to_squad_keep_source`], **not** the default
//! `move_slot_to_squad`, whose emptied-source branch garbage-collects the squad row, prunes it out
//! of `faction.squadIds` and deletes every vehicle attached to it. That is the right contract for a
//! one-slot drag-refile and exactly the wrong one here: "move these five to Alpha" is the operator
//! moving people, not disbanding Bravo and scrapping its transport.
//!
//! *Side keys are DERIVED, not stored.* `resolve_slot_side_key` walks `slot.squadId →
//! squad.factionId → faction.key`, so there is no side-key column for this module to write and no
//! way for it to forget one: rewriting `squadId` **is** the side change. The core's `SideKeyMemo` is
//! keyed by squad id and is invalidated by the `observe_after_transaction` bump every committed
//! transaction fires, so the next `materialize()` re-derives. The doc-level proof of both halves is
//! native and sits with the primitive, in `store.rs`'s own tests
//! (`keep_source_move_carries_the_derived_side_key_across_factions` and siblings).
#![cfg(target_arch = "wasm32")]

use super::batch::with_batch;
use super::context::{faction_rows, squad_rows, OPS_CTX};
use crate::editor::panels::attributes_modal::plan_reassign;
use crate::editor::state::history as mission_history;
use map_engine_core::doc::{MissionDocCore, NONE_IDX};

/// Where a batch reassign sends the selection.
///
/// `squad_id` empty means "this faction, its first squad" — the faction selector's commit. Keeping
/// the faction on the target even when a squad is named is what makes the cross-faction refusal
/// possible at all: without it the operation could not tell a deliberate pick from a squad that
/// drifted under another faction while the modal was open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReassignTarget {
    /// The faction picked in the modal (a `factionsById` row id).
    pub faction_id: String,
    /// The squad picked in the modal, or empty for "the faction's first squad".
    pub squad_id: String,
}

/// Move every id in `ids` into the squad `target` resolves to, as ONE undo group.
///
/// Returns the number of slots actually moved, or the refusal reason the modal shows. A slot
/// already in the destination is not a failure and not a move: the core's own no-op guard skips it
/// and it is not counted, so re-picking the current squad reports `0` rather than pretending.
///
/// The `with_batch` bracket is the whole reason this is one function and not a loop at the call
/// site: each id is its own core transaction, and without the group a five-slot reassign would cost
/// five Ctrl+Z presses to undo — the acceptance says one.
pub fn reassign_slots(ids: &[String], target: &ReassignTarget) -> Result<usize, String> {
    if ids.is_empty() {
        return Err("Nothing is selected.".to_string());
    }
    // Resolve BEFORE opening the undo group: a refusal must not leave an empty group behind.
    let dest = resolve_destination(target)?;
    let ids = ids.to_vec();
    let moved = with_batch("reassign-slots", move || {
        OPS_CTX.with(|c| {
            let guard = c.borrow();
            let Some(ctx) = guard.as_ref() else {
                return 0usize;
            };
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return 0usize;
            };
            // ONE `materialize()` for the whole batch, not one per id: it is O(all slots), and a
            // five-slot reassign paying it five times over is the kind of per-item full scan that
            // turns a 300-slot mission's undo group into a visible stall.
            let before = squads_by_slot(core);
            let mut moved = 0usize;
            for id in &ids {
                // The core no-ops a move into the squad the slot is already in; counting those
                // would report work that never happened, and the modal reports this number.
                match before.get(id.as_str()) {
                    None => continue, // gone from the doc since the selection was taken
                    Some(current) if current == &dest => continue,
                    Some(_) => {}
                }
                core.move_slot_to_squad_keep_source(id, &dest);
                moved += 1;
            }
            moved
        })
    });
    if moved > 0 {
        // Outside the borrow above, like every other mutator here: `after_local_edit` opens its own
        // read borrows (rebind + persist + the ORBAT rebuild the Outliner reads).
        mission_history::after_local_edit();
    }
    Ok(moved)
}

/// The live faction / squad rows the picker and the refusals are computed from, in one doc read.
#[must_use]
pub fn reassign_rows() -> (
    Vec<crate::editor::panels::outliner::FactionRow>,
    Vec<crate::editor::panels::outliner::SquadRow>,
) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        (faction_rows(core), squad_rows(core))
    })
}

/// `target` → destination squad id, or the refusal reason. Reads the rows live so the decision is
/// made against the document as it is now, not as the last render saw it.
fn resolve_destination(target: &ReassignTarget) -> Result<String, String> {
    let (factions, squads) = reassign_rows();
    if factions.is_empty() {
        return Err(
            "This mission has no factions yet — place a slot or add one in the ORBAT dock."
                .to_string(),
        );
    }
    plan_reassign(&factions, &squads, &target.faction_id, &target.squad_id)
}

/// `slot id → squad id` for every slot in the document, off ONE `materialize()`.
///
/// The SoA rather than the raw `slots_json`, because it is the same set `attrs_multi_ids` filtered
/// the selection against: an id missing here is an id the modal should not be writing to either.
fn squads_by_slot(core: &MissionDocCore) -> std::collections::HashMap<String, String> {
    let soa = core.materialize();
    soa.ids
        .iter()
        .enumerate()
        .map(|(row, id)| {
            let idx = soa.squad_idx[row];
            let squad = if idx == NONE_IDX {
                String::new()
            } else {
                soa.squads.get(idx as usize).cloned().unwrap_or_default()
            };
            (id.clone(), squad)
        })
        .collect()
}
