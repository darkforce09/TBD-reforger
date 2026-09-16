//! Role: the armed pointer-drag that moves a slot between squads.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: one thread-local armed slot id; the document is touched only on a drop.
//! Invariants: a refile is armed by exactly one pick-up and consumed by exactly one drop, so a
//! release outside a squad row leaves the order of battle untouched and the cell empty.

use std::cell::RefCell;

use super::MissionDocCore;

thread_local! {
    static PENDING_REFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Arm a slot for a refile into another squad.
pub fn begin_refile(slot_id: String) {
    PENDING_REFILE.with(|pending| *pending.borrow_mut() = Some(slot_id));
}

/// Clear an armed refile without touching the document — a release outside any squad row.
pub fn cancel_refile() {
    PENDING_REFILE.with(|pending| *pending.borrow_mut() = None);
}

/// Complete an armed refile onto `dest_squad_id`. `false` when nothing was armed.
pub fn complete_refile_onto_squad(core: &MissionDocCore, dest_squad_id: &str) -> bool {
    let Some(slot_id) = PENDING_REFILE.with(|pending| pending.borrow_mut().take()) else {
        return false;
    };
    refile_slot(core, &slot_id, dest_squad_id);
    true
}

/// Move a slot into `dest_squad_id` through the document's own `move_slot_to_squad` — the squad's
/// member list is the document's to splice, never a caller's.
pub fn refile_slot(core: &MissionDocCore, slot_id: &str, dest_squad_id: &str) {
    core.move_slot_to_squad(slot_id, dest_squad_id);
}

#[cfg(test)]
#[path = "tests/refile.rs"]
mod tests;
