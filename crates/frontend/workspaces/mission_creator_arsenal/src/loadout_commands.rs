//! Role: the Arsenal's writes to the mission document — commit one slot's loadout, apply the copy
//! buffer across a selection, and strip a selection back to nothing.
//! Position: the Arsenal crate's document seam, called by the loaded tab's handlers.
//! Signals & state: none of its own; the document and the selected ids come from the installed
//! editing host, and the copy buffer from the engine's loadout commands.
//! Invariants: the history tail fires only if the document ACKNOWLEDGED the write — a pick against
//! an id the mission no longer holds must not dirty the mission or mint an undo step that restores
//! nothing. Bulk gestures ask the operator first, through the host's own confirmation, and they
//! commit a plan this module made rather than writing the document row by row behind the shared
//! committer's back.

use mission_creator_engine_bridge::bridge::document_host::history as mission_history;
use mission_creator_engine_bridge::bridge::host_state::undo_grouped_gestures::confirm_bulk_n_step;
use mission_editing_commands::hosted_commands as engine_ops;
use mission_editing_session::host::with_doc;
use orbat_slot_ids::SlotUid;

/// Set or clear one slot's `loadout` and take the tail only on an acknowledged write. `None` or an
/// empty document clears the key. Returns the document's answer so a panel can surface a refusal.
pub fn set_loadout(id: &SlotUid, loadout_json: Option<String>) -> bool {
    let id = id.clone();
    crate::commit_one_write(
        || with_doc(|core| core.update_slot_loadout(id, loadout_json)).unwrap_or(false),
        || {
            mission_history::after_local_edit();
        },
    )
}

/// Apply the copy buffer across the selection: one buffered loadout per target, drawn from a seed
/// so the draw is reproducible for the whole gesture. Returns `(planned, committed)`; a refusal
/// from the compatibility rules comes back as the refused rows instead.
pub fn apply_loadout_buffer_to_selection(
    items: &[frontend_api_dtos::RegistryItem],
    feed: &mission_creator_state::arsenal_rules::CompatFeed,
) -> crate::error::Result<(usize, usize)> {
    let buffer = engine_ops::loadout_buffer();
    let targets = engine_ops::selection_slot_targets();
    if buffer.is_empty() || targets.is_empty() {
        return Ok((0, 0));
    }
    if !confirm_bulk_n_step(targets.len(), "overwrite the loadout of") {
        return Ok((0, 0));
    }
    let seed = engine_ops::next_apply_seed();
    let writes = crate::plan_apply(&targets, &buffer, seed, items, feed)?;
    Ok((writes.len(), engine_ops::commit_loadout_writes(&writes)))
}

/// Strip every selected entity's loadout back to the canonical empty document. Returns
/// `(planned, committed)`, for the same reason Apply does: a plan the document refused in part is
/// the operator's business.
pub fn remove_all_loadouts_from_selection() -> (usize, usize) {
    let targets = engine_ops::selection_slot_targets();
    if targets.is_empty() {
        return (0, 0);
    }
    if !confirm_bulk_n_step(targets.len(), "remove every item from") {
        return (0, 0);
    }
    let writes = crate::plan_remove(&targets);
    (writes.len(), engine_ops::commit_loadout_writes(&writes))
}
