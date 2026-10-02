//! Role: the editor's folder tree over the hosted document — create, rename, delete, reparent and
//! refile folders, the hide/lock flags a folder or a selection carries, and the pointer-drag that
//! moves a row between folders.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host and the armed tree drag is
//! held by the document operations this module rides.
//! Invariants: every mutator runs exactly one post-change tail, so a tree gesture is one Ctrl+Z,
//! and a call that changed nothing runs no tail at all. Which folder is ACTIVE is host state: it
//! crosses in as an argument and the new folder's id crosses back out, because the engine never
//! reads a dock's focus.

use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, with_doc};

use super::document_edit::commit_document_edit;

/// Hide or reveal one folder and everything filed under it.
pub fn set_layer_hidden(id: &str, hidden: bool) {
    commit_document_edit(|core| entity_ops::set_layer_hidden(core, id, hidden));
}

/// Lock or unlock one folder, so the rows under it refuse selection and drag.
pub fn set_layer_locked(id: &str, locked: bool) {
    commit_document_edit(|core| entity_ops::set_layer_locked(core, id, locked));
}

/// Hide or reveal every selected row in one transaction. `false` when the selection was empty or
/// already in that state, in which case no tail runs.
pub fn set_selection_hidden(hidden: bool) -> bool {
    let sel = selection_ids();
    let did = with_doc(|core| entity_ops::set_selection_hidden(core, hidden, sel)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Hide every selected row.
pub fn hide_selection() -> bool {
    set_selection_hidden(true)
}

/// Reveal every selected row.
pub fn show_selection() -> bool {
    set_selection_hidden(false)
}

/// Flip the selection's visibility as one act: an all-hidden selection reveals, anything else
/// hides. `false` when there is nothing to flip.
pub fn toggle_hidden() -> bool {
    let sel = selection_ids();
    match with_doc(|core| entity_ops::toggle_hidden(core, sel)).flatten() {
        Some(hidden) => set_selection_hidden(hidden),
        None => false,
    }
}

/// Clear every hidden flag in the document in one transaction. Returns how many rows were
/// revealed; zero runs no tail.
pub fn show_all_hidden() -> usize {
    let cleared = with_doc(entity_ops::show_all_hidden).unwrap_or(0);
    if cleared > 0 {
        after_local_edit();
    }
    cleared
}

/// Create a folder as a child of `active_layer` (a root when that is `None`), auto-named, with its
/// inline rename armed. Returns the new folder's id so the host can make it active.
pub fn create_layer(active_layer: Option<String>) -> Option<String> {
    let id = with_doc(|core| entity_ops::create_layer(core, active_layer));
    if id.is_some() {
        after_local_edit();
    }
    id
}

/// Take the id of the just-created folder whose inline rename should open, if any. Consumed once,
/// so a dock arms the input exactly once per creation.
#[must_use]
pub fn take_rename_armed() -> Option<String> {
    entity_ops::take_rename_armed()
}

/// Rename a folder. A blank name after trimming is refused — a folder must keep a label.
pub fn rename_layer(id: &str, name: &str) -> bool {
    let renamed = with_doc(|core| entity_ops::rename_layer(core, id, name)).unwrap_or(false);
    if renamed {
        after_local_edit();
    }
    renamed
}

/// Delete a folder AND its whole subtree — nested folders and every row filed in any of them. The
/// document keeps at least one folder, reseeding a default when the subtree was all of them.
pub fn delete_layer(id: &str) -> bool {
    commit_document_edit(|core| entity_ops::delete_layer(core, id))
}

/// Reparent a folder; `new_parent = None` moves it to the root. A drop into the folder's own
/// subtree is a no-op at the document, so this never re-checks for cycles.
pub fn reparent_layer(id: &str, new_parent: Option<String>) -> bool {
    commit_document_edit(|core| entity_ops::reparent_layer(core, id, new_parent))
}

/// Refile a row into a different folder: detach from every folder holding it, append to the
/// target. Squad membership is unchanged — filing is workflow, not ORBAT.
pub fn refile_slot_to_layer(slot_id: &str, layer_id: &str) -> bool {
    commit_document_edit(|core| entity_ops::refile_slot_to_layer(core, slot_id, layer_id))
}

/// Arm a folder for a pointer-drag reparent.
pub fn begin_layer_drag(layer_id: String) {
    entity_ops::begin_layer_drag(layer_id);
}

/// Arm a slot row for a pointer-drag refile into a folder.
pub fn begin_layer_slot_drag(slot_id: String) {
    entity_ops::begin_layer_slot_drag(slot_id);
}

/// Arm a comment row for a pointer-drag refile into a folder.
pub fn begin_layer_comment_drag(comment_id: String) {
    entity_ops::begin_layer_comment_drag(comment_id);
}

/// Drop an armed drag anywhere that is not a valid target: clear it without mutating.
pub fn cancel_layer_drag() {
    entity_ops::cancel_layer_drag();
}

/// Complete an armed drag onto a folder: a folder drag reparents under it, a row drag refiles into
/// it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(dest_folder_id: String) -> bool {
    let dropped =
        with_doc(|core| entity_ops::complete_layer_drop_onto_folder(core, &dest_folder_id))
            .unwrap_or(false);
    if dropped {
        after_local_edit();
    }
    dropped
}

/// Complete an armed FOLDER drag by reparenting it to the root. A row drag is dropped instead:
/// "filed under no folder" is not a state the document models, and the root dropzone is a
/// folder-reparent affordance. `false` when nothing was armed, or a row was.
pub fn complete_layer_drop_onto_root() -> bool {
    let dropped = with_doc(entity_ops::complete_layer_drop_onto_root).unwrap_or(false);
    if dropped {
        after_local_edit();
    }
    dropped
}
