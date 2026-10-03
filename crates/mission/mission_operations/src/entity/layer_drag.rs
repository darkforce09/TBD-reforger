//! Role: the armed pointer-drag inside the layer tree, from pick-up to drop.
//! Position: the `entity::layer_drag` module of `mission_operations`; hosted commands drive it.
//! Signals & state: one thread-local armed drag; the document is touched only on a drop.
//! Invariants: a drag is armed by exactly one pick-up and consumed by exactly one drop, so a
//! release anywhere that is not a valid target leaves the document untouched and the cell empty.

use mission_document::ids::{CommentId, LayerId};
use orbat_slot_ids::SlotUid;

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
pub fn begin_layer_drag(layer_id: impl Into<LayerId>) {
    let layer_id: LayerId = layer_id.into();
    let layer_id = layer_id.into_inner();
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Folder(layer_id)));
}

/// Arm a slot for a refile into a folder.
pub fn begin_layer_slot_drag(slot_id: impl Into<SlotUid>) {
    let slot_id: SlotUid = slot_id.into();
    let slot_id = slot_id.into_inner();
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Slot(slot_id)));
}

/// Arm a comment for a refile into a folder.
pub fn begin_layer_comment_drag(comment_id: impl Into<CommentId>) {
    let comment_id: CommentId = comment_id.into();
    let comment_id = comment_id.into_inner();
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = Some(LayerDrag::Comment(comment_id)));
}

/// Drop an armed drag anywhere that is not a valid target: clear it without touching the document.
pub fn cancel_layer_drag() {
    PENDING_LAYER_DRAG.with(|pending| *pending.borrow_mut() = None);
}

/// Complete an armed drag onto `dest_folder_id`. A folder reparents under it — dropping a folder
/// on itself is refused here, and a drop into its own subtree is refused by the document's cycle
/// guard. A slot or a comment refiles into it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(
    core: &MissionDocCore,
    dest_folder_id: impl Into<LayerId>,
) -> bool {
    let dest_folder_id: LayerId = dest_folder_id.into();
    let dest_folder_id = dest_folder_id.as_str();
    let Some(drag) = PENDING_LAYER_DRAG.with(|pending| pending.borrow_mut().take()) else {
        return false;
    };
    match drag {
        LayerDrag::Folder(id) => {
            if id == dest_folder_id {
                return false;
            }
            reparent_layer(core, id.as_str(), Some(dest_folder_id.to_string()));
        }
        LayerDrag::Slot(slot_id) => refile_slot_to_layer(core, slot_id.as_str(), dest_folder_id),
        // Comments are filed by the comment tree rather than by the slot tree, so this rides the
        // comment's own move rather than the slot refile above.
        LayerDrag::Comment(comment_id) => {
            core.move_comment_to_layer(comment_id.as_str(), dest_folder_id)
        }
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
            reparent_layer(core, id.as_str(), None);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/layer_drag.rs"]
mod tests;
