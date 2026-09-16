//! Role: the armed pointer-drag inside the layer tree, from pick-up to drop.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: one thread-local armed drag; the document is touched only on a drop.
//! Invariants: a drag is armed by exactly one pick-up and consumed by exactly one drop, so a
//! release anywhere that is not a valid target leaves the document untouched and the cell empty.

use std::cell::RefCell;

use super::MissionDocCore;
use super::refile_slot_to_layer;
use super::reparent_layer;

/// What a layer-tree row picked up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerDrag {
    /// A folder, to be reparented under whatever it is dropped on.
    Folder(String),

    /// A slot, to be refiled into the folder it is dropped on.
    Slot(String),

    /// A comment, to be refiled into the folder it is dropped on.
    Comment(String),
}

thread_local! {
    static PENDING_LAYER_DRAG: RefCell<Option<LayerDrag>> = const { RefCell::new(None) };
}

/// Arm a folder for a reparent.
pub fn begin_layer_drag(layer_id: String) {
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Folder(layer_id)));
}

/// Arm a slot for a refile into a folder.
pub fn begin_layer_slot_drag(slot_id: String) {
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Slot(slot_id)));
}

/// Arm a comment for a refile into a folder.
pub fn begin_layer_comment_drag(comment_id: String) {
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Comment(comment_id)));
}

/// Drop an armed drag anywhere that is not a valid target: clear it without touching the document.
pub fn cancel_layer_drag() {
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = None);
}

/// Complete an armed drag onto `dest_folder_id`. A folder reparents under it — dropping a folder
/// on itself is refused here, and a drop into its own subtree is refused by the document's cycle
/// guard. A slot or a comment refiles into it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(core: &MissionDocCore, dest_folder_id: &str) -> bool {
    let Some(drag) = PENDING_LAYER_DRAG.with(|pending| pending.borrow_mut().take()) else {
        return false;
    };
    match drag {
        LayerDrag::Folder(id) => {
            if id == dest_folder_id {
                return false;
            }
            reparent_layer(core, &id, Some(dest_folder_id.to_string()));
        }
        LayerDrag::Slot(slot_id) => refile_slot_to_layer(core, &slot_id, dest_folder_id),
        // Comments are filed by the comment tree rather than by the slot tree, so this rides the
        // comment's own move rather than the slot refile above.
        LayerDrag::Comment(comment_id) => core.move_comment_to_layer(&comment_id, dest_folder_id),
    }
    true
}

/// Complete an armed FOLDER drag by reparenting it to the root — the tree header's dropzone. An
/// armed slot or comment is DROPPED instead: every slot and comment lives in some folder, so
/// "refile to no folder" is not a state the document models, and the header is a folder-reparent
/// affordance. `false` when nothing was armed, or when what was armed was not a folder.
pub fn complete_layer_drop_onto_root(core: &MissionDocCore) -> bool {
    let drag = PENDING_LAYER_DRAG.with(|pending| pending.borrow_mut().take());
    match drag {
        Some(LayerDrag::Folder(id)) => {
            reparent_layer(core, &id, None);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/layer_drag.rs"]
mod tests;
