//! Role: the Arsenal's writes to the mission document — commit one slot's loadout, apply the copy
//! buffer across a selection, and strip a selection back to nothing.
//! Position: `editor/arsenal` in the frontend editor shell.
//! Signals & state: none of its own; the document and the selected ids come from the installed
//! editing host, and the copy buffer from the engine's loadout commands.
//! Invariants: the history tail fires only if the document ACKNOWLEDGED the write — a pick against
//! an id the mission no longer holds must not dirty the mission or mint an undo step that restores
//! nothing. Bulk gestures ask the operator first, through the host's own confirmation, and they
//! commit a plan this module made rather than writing the document row by row behind the shared
//! committer's back.

use crate::v2::apps::editor::bridge::document_host::history as mission_history;
use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures::confirm_bulk_n_step;
use website_map_engine::editing::host::with_doc;
use website_map_engine::editing::hosted_commands as engine_ops;

/// Set or clear one slot's `loadout` and take the tail only on an acknowledged write. `None` or an
/// empty document clears the key. Returns the document's answer so a panel can surface a refusal.
pub fn set_loadout(id: &str, loadout_json: Option<String>) -> bool {
    crate::v2::apps::editor::arsenal::commit_one_write(
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
    items: &[crate::v2::core::api::dto::RegistryItem],
    feed: &crate::v2::apps::editor::arsenal::arsenal_rules::CompatFeed,
) -> Result<(usize, usize), Vec<crate::v2::apps::editor::arsenal::arsenal_rules::RowError>> {
    let buffer = engine_ops::loadout_buffer();
    let targets = engine_ops::selection_slot_targets();
    if buffer.is_empty() || targets.is_empty() {
        return Ok((0, 0));
    }
    if !confirm_bulk_n_step(targets.len(), "overwrite the loadout of") {
        return Ok((0, 0));
    }
    let seed = engine_ops::next_apply_seed();
    let writes =
        crate::v2::apps::editor::arsenal::plan_apply(&targets, &buffer, seed, items, feed)?;
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
    let writes = crate::v2::apps::editor::arsenal::plan_remove(&targets);
    (writes.len(), engine_ops::commit_loadout_writes(&writes))
}
