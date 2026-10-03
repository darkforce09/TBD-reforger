//! **Role:** the ORBAT roster over the hosted document — squads, the slots in them, and the
//! vehicles attached to them.
//! **Position:** `hosted_commands::orbat_roster` in `mission_editing_commands`.
//! **Signals & state:** none of its own; the document and the id minter come from the host.
//! **Invariants:** every mutator runs exactly one post-change tail, and a call that changed nothing
//! runs none. Adding a role mints a slot on the SoA — it is not a place under a side, so a slot
//! added here carries its squad's anchor rather than a cursor position. The folder a new entity is
//! filed under is the HOST's answer and crosses as a closure, because which folder is active is
//! host state. Refusals that a surface renders are named sentences, never a silent `false`.

use mission_document::MissionDocCore;
use mission_document::ids::SquadId;
use mission_editing_session::history::after_local_edit;
use mission_editing_session::host::{with_doc, with_host};
use mission_operations::cargo::seed_cargo_for_asset;
use mission_operations::entity as entity_ops;
use mission_operations::faction_library::FactionDoc;
use mission_operations::rows::{FactionRow, SquadRow};
use orbat_slot_ids::SlotUid;

use super::document_edit::commit_document_edit;
use crate::error::{Error, Result};

use mission_operations::entity::OrbatManagerSnapshot;

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
    let slot_squad_ids = snap.slots.iter().map(|s| s.squad_id.to_string()).collect();
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
    squad_id: impl Into<SquadId>,
    role: String,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let squad_id: SquadId = squad_id.into();
    let id = with_host(|host| {
        let doc = host.doc.borrow();
        let core = doc.as_ref()?;
        entity_ops::orbat_add_slot(
            core,
            squad_id,
            role,
            &host.next_id,
            ensure_layer,
            |core, slot_id, asset_id, loadout| {
                seed_cargo_for_asset(core, slot_id, asset_id, loadout)
            },
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
pub fn orbat_set_leader(squad_id: impl Into<SquadId>, slot_id: impl Into<SlotUid>) -> bool {
    let squad_id: SquadId = squad_id.into();
    let slot_id: SlotUid = slot_id.into();
    commit_document_edit(|core| core.set_leader(squad_id.as_str(), slot_id.as_str()))
}

/// Remove a slot, detaching it from any vehicle seat, garbage-collecting a squad it leaves empty
/// and promoting a new leader when it was the leader.
pub fn orbat_remove_slot(slot_id: impl Into<SlotUid>) -> bool {
    let slot_id: SlotUid = slot_id.into();
    let did = with_doc(|core| entity_ops::orbat_remove_slot(core, slot_id)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Remove a squad and every slot in it.
pub fn orbat_remove_squad(squad_id: impl Into<SquadId>) -> bool {
    let squad_id: SquadId = squad_id.into();
    commit_document_edit(|core| core.remove_squad(squad_id.as_str()))
}

/// Rename a squad.
pub fn orbat_rename_squad(squad_id: impl Into<SquadId>, name: String) -> bool {
    let squad_id: SquadId = squad_id.into();
    commit_document_edit(|core| core.rename_squad(squad_id.as_str(), &name))
}

/// Write a whole faction library document onto `side`: its name, its roles as slots, and its
/// vehicles. The refusal's `Display` text is a sentence the caller renders verbatim:
/// [`Error::NoEditorOpen`], [`Error::NoDocumentLoaded`] or the document's own
/// [`Error::FactionRefused`].
pub fn orbat_apply_faction(
    side: String,
    doc: FactionDoc,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Result<()> {
    let res = with_host(|host| {
        let borrowed = host.doc.borrow();
        let Some(core) = borrowed.as_ref() else {
            return Err(Error::NoDocumentLoaded);
        };
        entity_ops::orbat_apply_faction(
            core,
            side,
            doc,
            ensure_layer,
            |core, slot_id, asset_id, loadout| {
                seed_cargo_for_asset(core, slot_id, asset_id, loadout)
            },
        )
        .map_err(Error::FactionRefused)
    })
    .unwrap_or(Err(Error::NoEditorOpen));
    if res.is_ok() {
        after_local_edit();
    }
    res
}

/// Attach a vehicle to a squad, anchored beside it. A blank resource name is refused before the
/// document is opened at all — an unnamed vehicle has nothing to spawn.
pub fn orbat_add_vehicle(squad_id: impl Into<SquadId>, resource_name: &str) -> Option<String> {
    if resource_name.trim().is_empty() {
        return None;
    }
    let squad_id: SquadId = squad_id.into();
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

/// The live side projection of a faction, derived from the document's own squads and slots. A save
/// caller must combine it with the stored library through `merge_faction_doc_from_side` to retain
/// library-only fields and vehicle labels.
#[must_use]
pub fn faction_doc_from_side(side: &str) -> Option<FactionDoc> {
    with_doc(|core| entity_ops::faction_doc_from_side_core(core, side)).flatten()
}

/// Patch a slot's inspector fields: role and tag through the slot record, callsign and rank through
/// its identity. `false` when nothing changed, which runs no tail.
pub fn orbat_update_slot_fields(
    slot_id: impl Into<SlotUid>,
    role: Option<String>,
    tag: Option<String>,
    callsign: Option<String>,
    rank: Option<String>,
) -> bool {
    let slot_id: SlotUid = slot_id.into();
    let did = with_doc(|core| {
        entity_ops::orbat_update_slot_fields(core, slot_id, role, tag, callsign, rank)
    })
    .unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Arm a slot for a pointer-drag refile into another squad.
pub fn begin_refile(slot_id: impl Into<SlotUid>) {
    entity_ops::begin_refile(slot_id);
}

/// Clear an armed refile without mutating the document — a drop outside any squad row.
pub fn cancel_refile() {
    entity_ops::cancel_refile();
}

/// Complete an armed refile onto a squad. `false` when nothing was armed.
pub fn complete_refile_onto_squad(dest_squad_id: impl Into<SquadId>) -> bool {
    let dest_squad_id: SquadId = dest_squad_id.into();
    let did = with_doc(|core| entity_ops::complete_refile_onto_squad(core, dest_squad_id.as_str()))
        .unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Move one slot into another squad. Squad membership is the document's own link, so this is one
/// transaction and one undo step.
pub fn refile_slot(slot_id: impl Into<SlotUid>, dest_squad_id: impl Into<SquadId>) -> bool {
    let slot_id: SlotUid = slot_id.into();
    let dest_squad_id: SquadId = dest_squad_id.into();
    commit_document_edit(|core| {
        entity_ops::refile_slot(core, slot_id.as_str(), dest_squad_id.as_str())
    })
}

/// Drop one slot onto another so it joins that slot's squad. Refused — with no tail — when the
/// target is the dragged slot itself, when the target has no squad (an unfiled slot, or a target
/// that has gone), or when the two already share a squad: a same-squad move is a no-op at the
/// document, and declining here keeps the caller from firing a dirty tail for nothing.
pub fn regroup_slot_onto(slot_id: impl Into<SlotUid>, target_id: impl Into<SlotUid>) -> bool {
    let slot_id: SlotUid = slot_id.into();
    let target_id: SlotUid = target_id.into();
    if slot_id == target_id {
        return false;
    }
    let dest_squad = super::slot_attributes::read_attrs(target_id)
        .map(|a| a.squad)
        .unwrap_or_default();
    let src_squad = super::slot_attributes::read_attrs(slot_id.clone())
        .map(|a| a.squad)
        .unwrap_or_default();
    if dest_squad.is_empty() || dest_squad == src_squad {
        return false;
    }
    refile_slot(slot_id, dest_squad)
}
