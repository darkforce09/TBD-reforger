//! Role: the armed pointer-drag that moves a slot between squads.
//! Position: the `entity::refile` module of `mission_operations`; hosted commands drive it.
//! Signals & state: one thread-local armed slot id; the document is touched only on a drop.
//! Invariants: a refile is armed by exactly one pick-up and consumed by exactly one drop, so a
//! release outside a squad row leaves the order of battle untouched and the cell empty.

use mission_document::ids::SquadId;
use orbat_slot_ids::SlotUid;

use std::cell::RefCell;

use super::MissionDocCore;

thread_local! {
    static PENDING_REFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Arm a slot for a refile into another squad.
pub fn begin_refile(slot_id: impl Into<SlotUid>) {
    let slot_id: SlotUid = slot_id.into();
    let slot_id = slot_id.into_inner();
    PENDING_REFILE.with(|pending| *pending.borrow_mut() = Some(slot_id));
}

/// Clear an armed refile without touching the document — a release outside any squad row.
pub fn cancel_refile() {
    PENDING_REFILE.with(|pending| *pending.borrow_mut() = None);
}

/// Complete an armed refile onto `dest_squad_id`. `false` when nothing was armed.
pub fn complete_refile_onto_squad(
    core: &MissionDocCore,
    dest_squad_id: impl Into<SquadId>,
) -> bool {
    let dest_squad_id: SquadId = dest_squad_id.into();
    let dest_squad_id = dest_squad_id.as_str();
    let Some(slot_id) = PENDING_REFILE.with(|pending| pending.borrow_mut().take()) else {
        return false;
    };
    refile_slot(core, slot_id.as_str(), dest_squad_id);
    true
}

/// Move a slot into `dest_squad_id` through the document's own `move_slot_to_squad` — the squad's
/// member list is the document's to splice, never a caller's.
pub fn refile_slot(
    core: &MissionDocCore,
    slot_id: impl Into<SlotUid>,
    dest_squad_id: impl Into<SquadId>,
) {
    let dest_squad_id: SquadId = dest_squad_id.into();
    let dest_squad_id = dest_squad_id.as_str();
    let slot_id: SlotUid = slot_id.into();
    let slot_id = slot_id.as_str();
    core.move_slot_to_squad(slot_id, dest_squad_id);
}
