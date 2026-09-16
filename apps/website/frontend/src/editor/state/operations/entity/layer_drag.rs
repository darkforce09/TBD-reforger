//! Role: layer drag.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Arm a folder for a pointer-drag reparent (folder-row `pointerdown`).
pub fn begin_layer_drag(layer_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(layer_id)));
}

/// Arm a slot for a pointer-drag refile into a folder (slot-row `pointerdown`, layer tree only).
pub fn begin_layer_slot_drag(slot_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(slot_id)));
}

/// Begin layer comment drag using the supplied domain data.
pub fn begin_layer_comment_drag(comment_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(comment_id)));
}

/// Drop an armed drag ANYWHERE that isn't a valid target (clear without mutating).
pub fn cancel_layer_drag() {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed drag onto `dest_folder_id`: a FOLDER drag reparents under it (no-op self/subtree drop — the core rejects it); a SLOT drag refiles into it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(dest_folder_id: String) -> bool {
    let Some(drag) = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    match drag {
        LayerDrag::Folder(id) => {
            if id == dest_folder_id {
                return false;
            }
            reparent_layer(&id, Some(dest_folder_id))
        }
        LayerDrag::Slot(slot_id) => refile_slot_to_layer(&slot_id, &dest_folder_id),

        LayerDrag::Comment(comment_id) => refile_comment_to_layer(&comment_id, &dest_folder_id),
    }
}

/// Complete an armed FOLDER drag by reparenting it to the ROOT (the header dropzone). A slot drag is dropped (a slot must live in some folder; "refile to no folder" is not a thing the doc models — it would just leave the slot unfiled, and the root dropzone is a folder-reparent affordance). `false` when nothing was armed or a slot was armed.
pub fn complete_layer_drop_onto_root() -> bool {
    let drag = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take());
    match drag {
        Some(LayerDrag::Folder(id)) => reparent_layer(&id, None),
        _ => false,
    }
}

/// SEL-LAYER-CHILDREN-001 — select a folder's DIRECT slot children (replacing the selection).
pub fn select_layer_children(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(
            crate::editor::panels::outliner_tree::layer_direct_slot_children(
                &layer_rows(core),
                layer_id,
            ),
        )
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// SEL-LAYER-DESC-001 — select every slot in a folder's whole subtree (replacing the selection).
pub fn select_layer_descendants(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(
            crate::editor::panels::outliner_tree::layer_descendant_slots(
                &layer_rows(core),
                layer_id,
            ),
        )
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// Set slot selection using the supplied domain data.
pub(in crate::editor::state::operations) fn set_slot_selection(ids: Vec<String>) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        *ctx.selection.borrow_mut() = ids;
        let ids = ctx.selection.borrow().clone();
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids);
        }
    });
    mission_history::refresh_selection();
}
