//! Role: authored layer folders — create, rename, delete, reparent, refile, and visibility.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs, plus two thread-local session cells — the layer-id
//! counter every mint advances, and the folder whose inline rename should open next.
//! Invariants: preserve authored order, numeric precision, and wire representations. Each entry
//! point is ONE authored transaction, so a host that groups undo steps gets one step per call.

use std::cell::{Cell, RefCell};

use super::MissionDocCore;
use super::layer_rows;
use super::mint_layer_id;
use super::mint_layer_name;

thread_local! {
    /// Monotonic counter behind every layer-id mint. It is a starting point, not a guarantee:
    /// [`mint_layer_id`] still proves the id it returns is unused by the live document, because
    /// undo frees ids and a restore can bring back a document that already used one.
    static NEXT_LAYER_ID: Cell<u32> = const { Cell::new(0) };

    /// The folder whose inline rename should open, consumed once by [`take_rename_armed`] so a
    /// host arms its input exactly once per creation.
    static ARMED_RENAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Hide or reveal one folder and everything filed under it.
pub fn set_layer_hidden(core: &MissionDocCore, id: &str, hidden: bool) {
    core.set_editor_layer_hidden(id, hidden);
}

/// Lock or unlock one folder against selection and transform.
pub fn set_layer_locked(core: &MissionDocCore, id: &str, locked: bool) {
    core.set_editor_layer_locked(id, locked);
}

/// Clear every hidden flag in the document in one transaction. Returns how many were cleared, so
/// a caller can tell a real reveal from a no-op.
pub fn show_all_hidden(core: &MissionDocCore) -> usize {
    core.clear_all_editor_hidden()
}

/// Create a folder as a CHILD of `active_layer` (a root when that is `None` or names a folder the
/// document no longer holds), auto-named by [`mint_layer_name`], and arm its inline rename.
/// Returns the new id.
pub fn create_layer(core: &MissionDocCore, active_layer: Option<String>) -> String {
    let rows = layer_rows(core);
    let parent = active_layer.filter(|active| rows.iter().any(|l| &l.id == active));
    let id = NEXT_LAYER_ID.with(|next_id| mint_layer_id(core, next_id));
    let name = mint_layer_name(core);
    core.add_editor_layer(&id, &name, parent);
    ARMED_RENAME.with(|armed| *armed.borrow_mut() = Some(id.clone()));
    id
}

/// Take the id of the just-created folder whose inline rename should open, if any. Consumed on
/// read, so the rename opens once and a later read reports nothing armed.
#[must_use]
pub fn take_rename_armed() -> Option<String> {
    ARMED_RENAME.with(|armed| armed.borrow_mut().take())
}

/// Rename a folder. A name that is blank after trimming is REFUSED — a folder must keep a label —
/// and the document is left untouched.
pub fn rename_layer(core: &MissionDocCore, id: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    core.rename_editor_layer(id, name);
    true
}

/// Delete a folder with subtree semantics: `remove_editor_layer` takes the folder AND everything
/// filed beneath it (child folders and their slots), keeping at least one layer alive by reseeding
/// a fresh default when the subtree was every layer the document had.
pub fn delete_layer(core: &MissionDocCore, id: &str) {
    let reseed = NEXT_LAYER_ID.with(|next_id| mint_layer_id(core, next_id));
    core.remove_editor_layer(id, &reseed);
}

/// Move a folder under `new_parent`, or to the root when that is `None`. The cycle guard lives in
/// `reparent_editor_layer` — a drop into the folder's own subtree is a no-op there — so this does
/// not re-check it.
pub fn reparent_layer(core: &MissionDocCore, id: &str, new_parent: Option<String>) {
    core.reparent_editor_layer(id, new_parent);
}

/// Move a slot into a different folder: detach it from every folder holding it, then append it to
/// the target. Squad membership is untouched — filing is a workflow, not an order of battle.
pub fn refile_slot_to_layer(core: &MissionDocCore, slot_id: &str, layer_id: &str) {
    core.move_slot_to_layer(slot_id, layer_id);
}

/// Which folder a placement lands in, and whether the caller's active-layer reading was stale.
pub struct EnsuredLayer {
    /// The folder the placement files into.
    pub layer_id: String,

    /// The caller named an active layer the document no longer holds. Whoever tracks that reading
    /// clears it; this side does not know where it is kept.
    pub active_layer_was_stale: bool,
}

/// Resolve the folder a placement files into: the caller's `active_layer` when the document still
/// holds it, else the first authored folder, else a freshly seeded default named by `default_id`
/// and `default_name` (the caller supplies those, because what a default folder is called is the
/// host's vocabulary, not the document's).
pub fn ensure_layer(
    core: &MissionDocCore,
    active_layer: Option<String>,
    default_id: &str,
    default_name: &str,
) -> EnsuredLayer {
    let rows = layer_rows(core);
    let mut active_layer_was_stale = false;
    if let Some(active) = active_layer {
        if rows.iter().any(|l| l.id == active) {
            return EnsuredLayer {
                layer_id: active,
                active_layer_was_stale,
            };
        }
        active_layer_was_stale = true;
    }
    let layer_id = if let Some(first) = rows.first() {
        first.id.clone()
    } else {
        core.add_editor_layer(default_id, default_name, None);
        default_id.to_string()
    };
    EnsuredLayer {
        layer_id,
        active_layer_was_stale,
    }
}

#[cfg(test)]
#[path = "tests/layers.rs"]
mod tests;
