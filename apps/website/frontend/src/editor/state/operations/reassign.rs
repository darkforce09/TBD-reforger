//! Role: reassign.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

#![cfg(target_arch = "wasm32")]

use super::batch::with_batch;
use super::context::{faction_rows, squad_rows, OPS_CTX};
use crate::editor::panels::attributes_modal::plan_reassign;
use crate::editor::state::history as mission_history;

/// Expose website mission core :: doc :: operations :: reassign ::  reassign target at this domain boundary.
pub use website_map_engine::data::store::operations::reassign::ReassignTarget;

/// Move every id in `ids` into the squad `target` resolves to, as ONE undo group.
pub fn reassign_slots(ids: &[String], target: &ReassignTarget) -> Result<usize, String> {
    if ids.is_empty() {
        return Err("Nothing is selected.".to_string());
    }

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
            website_map_engine::data::store::operations::reassign::reassign_slots(core, ids, dest)
        })
    });
    if moved > 0 {
        mission_history::after_local_edit();
    }
    Ok(moved)
}

/// Restore each slot's own on-open squad, including a selection split across factions/squads. This adds one membership undo group after Revert's existing transform/identity writes. Slots already home contribute no writes or history tail. Keep both ends of each move, including any vehicles authored on a destination that becomes empty again during Revert.
pub fn restore_slot_squads(snapshot: &[super::attrs::SlotAttrs]) -> usize {
    let moves = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::reassign::restore_moves(core, snapshot)
    });
    let moves = moves.unwrap_or_default();
    if moves.is_empty() {
        return 0;
    }
    let moved = with_batch("revert-slot-squads", || {
        OPS_CTX.with(|c| {
            let guard = c.borrow();
            let Some(ctx) = guard.as_ref() else {
                return 0;
            };
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return 0;
            };
            website_map_engine::data::store::operations::reassign::restore_slot_squads(core, moves)
        })
    });
    if moved > 0 {
        mission_history::after_local_edit();
    }
    moved
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
