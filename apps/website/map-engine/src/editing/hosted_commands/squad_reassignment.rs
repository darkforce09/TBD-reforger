//! Role: move a whole selection into another faction's squad over the hosted document, and put it
//! back.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: a batch reassign is ONE undo group — the per-slot document transactions are
//! bracketed, so one Ctrl+Z returns every moved slot. It takes the ADDITIVE keep-source move: a
//! slot leaving a squad never deletes that squad, its place in `faction.squadIds`, or the vehicles
//! attached to it. Refusals are named sentences the caller can render, never a silent no-op.

use crate::data::store::operations::attrs::SlotAttrs;
use crate::data::store::operations::projections::{faction_rows, squad_rows};
use crate::data::store::operations::reassign;
use crate::data::store::operations::rows::{FactionRow, SquadRow};
use crate::editing::batch::with_batch;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

/// The faction / squad a reassign is aimed at, as the picker names it.
pub use crate::data::store::operations::reassign::ReassignTarget;

/// Move every id in `ids` into the squad `target` resolves to, as ONE undo group.
pub fn reassign_slots(ids: &[String], target: &ReassignTarget) -> Result<usize, String> {
    if ids.is_empty() {
        return Err("Nothing is selected.".to_string());
    }

    let dest = resolve_destination(target)?;
    let ids = ids.to_vec();
    let moved = with_batch("reassign-slots", move || {
        with_doc(|core| reassign::reassign_slots(core, ids, dest)).unwrap_or(0)
    });
    if moved > 0 {
        after_local_edit();
    }
    Ok(moved)
}

/// Restore each slot's own on-open squad, including a selection split across factions and squads.
/// This adds one membership undo group after the transform and identity writes a revert already
/// made. Slots already home contribute no writes and no tail. Both ends of each move are kept,
/// including any vehicles authored on a destination that becomes empty again during the revert.
pub fn restore_slot_squads(snapshot: &[SlotAttrs]) -> usize {
    let moves = with_doc(|core| reassign::restore_moves(core, snapshot))
        .flatten()
        .unwrap_or_default();
    if moves.is_empty() {
        return 0;
    }
    let moved = with_batch("revert-slot-squads", || {
        with_doc(|core| reassign::restore_slot_squads(core, moves)).unwrap_or(0)
    });
    if moved > 0 {
        after_local_edit();
    }
    moved
}

/// The live faction / squad rows the picker and the refusals are computed from, in one read.
#[must_use]
pub fn reassign_rows() -> (Vec<FactionRow>, Vec<SquadRow>) {
    with_doc(|core| (faction_rows(core), squad_rows(core))).unwrap_or_default()
}

fn resolve_destination(target: &ReassignTarget) -> Result<String, String> {
    let (factions, squads) = reassign_rows();
    if factions.is_empty() {
        return Err(
            "This mission has no factions yet — place a slot or add one in the ORBAT dock."
                .to_string(),
        );
    }
    reassign::plan_reassign(&factions, &squads, &target.faction_id, &target.squad_id)
}
