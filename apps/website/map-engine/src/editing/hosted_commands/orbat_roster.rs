//! Role: the ORBAT roster over the hosted document — squads, the slots in them, and the vehicles
//! attached to them.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document and the id minter come from the host.
//! Invariants: every mutator runs exactly one post-change tail, and a call that changed nothing
//! runs none. Adding a role mints a slot on the SoA — it is not a place under a side, so a slot
//! added here carries its squad's anchor rather than a cursor position. The folder a new entity is
//! filed under is the HOST's answer and crosses as a closure, because which folder is active is
//! host state. Refusals that a surface renders are named sentences, never a silent `false`.

use crate::data::store::MissionDocCore;
use crate::data::store::operations::cargo::seed_cargo_for_asset;
use crate::data::store::operations::entity as entity_ops;
use crate::data::store::operations::faction_library::FactionDoc;
use crate::data::store::operations::rows::{FactionRow, SquadRow};
use crate::editing::history::after_local_edit;
use crate::editing::host::{with_doc, with_host};

use super::document_edit::commit_document_edit;

/// The live ORBAT rows plus each slot's loadout and identity, read once.
pub use crate::data::store::operations::entity::OrbatManagerSnapshot;

/// The whole roster in one read: factions, squads, and each slot's identity and loadout.
#[must_use]
pub fn orbat_manager_snapshot() -> OrbatManagerSnapshot {
    with_doc(entity_ops::orbat_manager_snapshot).unwrap_or_default()
}

/// The faction rows, the squad rows, and one squad id per slot — the census the Outliner header
/// and the ORBAT strip count from. Derived from [`orbat_manager_snapshot`] rather than from a
/// second document read, so the three cannot disagree about the same document.
#[must_use]
pub fn census_input() -> (Vec<FactionRow>, Vec<SquadRow>, Vec<String>) {
    let snap = orbat_manager_snapshot();
    let slot_squad_ids = snap.slots.iter().map(|s| s.squad_id.clone()).collect();
    (snap.factions, snap.squads, slot_squad_ids)
}

/// Add an empty squad under `side` (`BLUFOR` / `OPFOR` / `INDFOR`), ensuring that side's faction
/// first. `None` when the side is not one the document models.
pub fn orbat_add_squad(side: String) -> Option<String> {
    let id = with_doc(|core| entity_ops::orbat_add_squad(core, side)).flatten();
    if id.is_some() {
        after_local_edit();
    }
    id
}

/// Add a role to an existing squad, defaulting to Rifleman. `None` when the squad is gone.
pub fn orbat_add_slot(
    squad_id: String,
    role: String,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let id = with_host(|host| {
        let doc = host.doc.borrow();
        let core = doc.as_ref()?;
        entity_ops::orbat_add_slot(
            core,
            squad_id,
            role,
            &host.next_id,
            ensure_layer,
            seed_cargo_for_asset,
        )
    })
    .flatten();
    if id.is_some() {
        after_local_edit();
    }
    id
}

/// Make `slot_id` the leader of `squad_id`. The slot keeps whatever medic / engineer tag it
/// already carries — leadership and speciality are separate authored fields.
pub fn orbat_set_leader(squad_id: String, slot_id: String) -> bool {
    commit_document_edit(|core| core.set_leader(&squad_id, &slot_id))
}

/// Remove a slot, detaching it from any vehicle seat, garbage-collecting a squad it leaves empty
/// and promoting a new leader when it was the leader.
pub fn orbat_remove_slot(slot_id: String) -> bool {
    let did = with_doc(|core| entity_ops::orbat_remove_slot(core, slot_id)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Remove a squad and every slot in it.
pub fn orbat_remove_squad(squad_id: String) -> bool {
    commit_document_edit(|core| core.remove_squad(&squad_id))
}

/// Rename a squad.
pub fn orbat_rename_squad(squad_id: String, name: String) -> bool {
    commit_document_edit(|core| core.rename_squad(&squad_id, &name))
}

/// Write a whole faction library document onto `side`: its name, its roles as slots, and its
/// vehicles. The refusal is a sentence the caller renders verbatim.
pub fn orbat_apply_faction(
    side: String,
    doc: FactionDoc,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Result<(), String> {
    let res = with_host(|host| {
        let borrowed = host.doc.borrow();
        let Some(core) = borrowed.as_ref() else {
            return Err("No mission document is loaded.".to_string());
        };
        entity_ops::orbat_apply_faction(core, side, doc, ensure_layer, seed_cargo_for_asset)
    })
    .unwrap_or_else(|| Err("No mission editor is open.".to_string()));
    if res.is_ok() {
        after_local_edit();
    }
    res
}

/// Attach a vehicle to a squad, anchored beside it. A blank resource name is refused before the
/// document is opened at all — an unnamed vehicle has nothing to spawn.
pub fn orbat_add_vehicle(squad_id: String, resource_name: &str) -> Option<String> {
    if resource_name.trim().is_empty() {
        return None;
    }
    let id = with_host(|host| {
        let doc = host.doc.borrow();
        let core = doc.as_ref()?;
        entity_ops::orbat_add_vehicle(core, squad_id, resource_name, &host.next_id)
    })
    .flatten();
    if id.is_some() {
        after_local_edit();
    }
    id
}
