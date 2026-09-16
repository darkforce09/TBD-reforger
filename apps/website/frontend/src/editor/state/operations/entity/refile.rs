//! Role: refile.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Derive the live side projection. Save callers must combine it with the stored library
/// through `merge_faction_doc_from_side` to retain library-only fields and vehicle labels.
pub fn faction_doc_from_side(side: &str) -> Option<FactionDoc> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        faction_doc_from_side_core(core, side)
    })
}

/// Patch inspector fields: role/tag via `update_slot`, callsign/rank via `update_slot_identity`.
pub fn orbat_update_slot_fields(
    slot_id: String,
    role: Option<String>,
    tag: Option<String>,
    callsign: Option<String>,
    rank: Option<String>,
) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_map_engine::data::store::operations::entity::orbat_update_slot_fields(
            core, slot_id, role, tag, callsign, rank,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Arm a slot for refile into another squad (OrbatManager pointer-drag).
pub fn begin_refile(slot_id: String) {
    PENDING_REFILE.with(|p| *p.borrow_mut() = Some(slot_id));
}

/// Clear an armed refile without mutating the doc (drop outside a squad row).
pub fn cancel_refile() {
    PENDING_REFILE.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed refile onto `dest_squad_id` via [`refile_slot`].
pub fn complete_refile_onto_squad(dest_squad_id: String) -> bool {
    let Some(slot_id) = PENDING_REFILE.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    refile_slot(slot_id, dest_squad_id)
}

/// Move `slot_id` into `dest_squad_id` through core [`MissionDocCore::move_slot_to_squad`] only (F-L2 — no FE `slotIds` splice), then the shared dirty tail (orbat_nodes + squad links).
pub fn refile_slot(slot_id: String, dest_squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_squad(&slot_id, &dest_squad_id);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
