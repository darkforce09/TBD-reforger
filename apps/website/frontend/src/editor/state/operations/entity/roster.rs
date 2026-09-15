//! Role: roster.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Read live ORBAT rows + per-slot loadout/identity for the Stitch manager (G7 live data).
pub fn orbat_manager_snapshot() -> OrbatManagerSnapshot {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        website_mission_core::doc::operations::entity::orbat_manager_snapshot(core)
    })
}

/// Census input using the supplied domain data.
#[must_use]
pub fn census_input() -> (
    Vec<crate::editor::panels::outliner::FactionRow>,
    Vec<crate::editor::panels::outliner::SquadRow>,
    Vec<String>,
) {
    let snap = orbat_manager_snapshot();
    let slot_squad_ids = snap.slots.iter().map(|s| s.squad_id.clone()).collect();
    (snap.factions, snap.squads, slot_squad_ids)
}

/// G5 — add an empty squad under `side` (`BLUFOR`/`OPFOR`/`INDFOR`).
pub fn orbat_add_squad(side: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        website_mission_core::doc::operations::entity::orbat_add_squad(core, side)
    });
    if id.is_some() {
        mission_history::after_local_edit();
    }
    id
}

/// G6 — add a role (slot) into an existing squad; default role Rifleman. Not `place_character_under_side`.
pub fn orbat_add_slot(squad_id: String, role: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        website_mission_core::doc::operations::entity::orbat_add_slot(
            core,
            squad_id,
            role,
            &ctx.next_id,
            |core| ensure_layer(ctx, core),
            seed_cargo_in_core,
        )
    });
    if id.is_some() {
        mission_history::after_local_edit();
    }
    id
}

/// G2 — Make SL via core `set_leader` (does not overwrite MED/ENG tag).
pub fn orbat_set_leader(squad_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_leader(&squad_id, &slot_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Remove a slot (cascade detach); GC empty squad; promote leader when needed.
pub fn orbat_remove_slot(slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_mission_core::doc::operations::entity::orbat_remove_slot(core, slot_id)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Remove a squad and its slots.
pub fn orbat_remove_squad(squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_squad(&squad_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Rename a squad.
pub fn orbat_rename_squad(squad_id: String, name: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_squad(&squad_id, &name);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Orbat apply faction using the supplied domain data.
pub fn orbat_apply_faction(side: String, doc: FactionDoc) -> Result<(), String> {
    let res = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Err("No mission editor is open.".to_string());
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Err("No mission document is loaded.".to_string());
        };
        website_mission_core::doc::operations::entity::orbat_apply_faction(
            core,
            side,
            doc,
            |core| ensure_layer(ctx, core),
            seed_cargo_in_core,
        )
    });
    if res.is_ok() {
        mission_history::after_local_edit();
    }
    res
}

/// Orbat add vehicle using the supplied domain data.
pub fn orbat_add_vehicle(squad_id: String, resource_name: String) -> Option<String> {
    if resource_name.trim().is_empty() {
        return None;
    }
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        website_mission_core::doc::operations::entity::orbat_add_vehicle(
            core,
            squad_id,
            &resource_name,
            &ctx.next_id,
        )
    });
    if id.is_some() {
        mission_history::after_local_edit();

        crate::editor::panels::dock_right::record_placed(
            resource_name.clone(),
            resource_name.clone(),
        );
    }
    id
}
