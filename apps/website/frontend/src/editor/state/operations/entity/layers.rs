//! Role: layers.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Set active layer using the supplied domain data.
pub fn set_active_layer(id: Option<String>) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.active_layer.set(id);
        }
    });
}

/// Set layer hidden using the supplied domain data.
pub fn set_layer_hidden(id: &str, hidden: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_hidden(id, hidden);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Set layer locked using the supplied domain data.
pub fn set_layer_locked(id: &str, locked: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_locked(id, locked);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Set selection hidden using the supplied domain data.
pub(in crate::editor::state::operations) fn set_selection_hidden(hidden: bool) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_map_engine::data::store::operations::entity::set_selection_hidden(core, hidden, sel)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Hide selection using the supplied domain data.
#[allow(dead_code)]
pub fn hide_selection() -> bool {
    set_selection_hidden(true)
}

/// `#[allow(dead_code)]` — same residue note as [`hide_selection`] (the context-menu Show row / the H-key toggle live outside `owns`).
#[allow(dead_code)]
pub fn show_selection() -> bool {
    set_selection_hidden(false)
}

/// Toggle hidden using the supplied domain data.
#[allow(dead_code)]
pub fn toggle_hidden() -> bool {
    let dir = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::toggle_hidden(core, sel)
    });
    match dir {
        Some(hidden) => set_selection_hidden(hidden),
        None => false,
    }
}

/// `#[allow(dead_code)]`: a menu/command entry point (a "Show All" item) is the residue; the reveal- all txn + undo are proven at the store (`show_all_clears_every_flag_in_one_txn`).
#[allow(dead_code)]
pub fn show_all_hidden() -> usize {
    let cleared = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.clear_all_editor_hidden()
    });
    if cleared > 0 {
        mission_history::after_local_edit();
    }
    cleared
}

/// Domain representation of layer drag.
#[derive(Clone)]
pub(in crate::editor::state::operations) enum LayerDrag {
    /// A folder being reparented.
    Folder(String),

    /// A slot being refiled into a folder.
    Slot(String),

    /// Domain representation of comment.
    Comment(String),
}

/// LAYER-CREATE-001 — create a folder as a **child of the selected/active folder** (or a root when none is active), auto-named "New Layer N", and arm its inline rename. Returns the new id.
pub fn create_layer() -> Option<String> {
    let created = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let id = {
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let rows = layer_rows(core);

            let parent = ctx
                .active_layer
                .get_untracked()
                .filter(|a| rows.iter().any(|l| &l.id == a));
            let id = NEXT_LAYER_ID.with(|next_id| mint_layer_id(core, next_id));
            let name = mint_layer_name(core);
            core.add_editor_layer(&id, &name, parent);
            id
        };

        ctx.active_layer.set(Some(id.clone()));
        RENAME_ARMED.with(|r| *r.borrow_mut() = Some(id.clone()));
        Some(id)
    });
    if created.is_some() {
        mission_history::after_local_edit();
    }
    created
}

/// LAYER-CREATE-001 — take the id of the just-created layer whose inline rename should open, if any. Consumed once (cleared on read) so the dock arms the input exactly once per creation.
#[must_use]
pub fn take_rename_armed() -> Option<String> {
    RENAME_ARMED.with(|r| r.borrow_mut().take())
}

/// Rename an Outliner folder (inline-rename commit). Rides the shipped `rename_editor_layer`; one transaction ⇒ one undo step. A blank name after trim is rejected (a folder must keep a label).
pub fn rename_layer(id: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_editor_layer(id, name);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// LAYER-DEL-001 — delete a folder with the SHIPPED subtree semantics: `remove_editor_layer` deletes the folder AND its whole subtree (child folders + every slot filed in any of them), keeps ≥1 layer (reseeding a default if the subtree was every layer). One transaction ⇒ one undo step.
pub fn delete_layer(id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let reseed = NEXT_LAYER_ID.with(|next_id| mint_layer_id(core, next_id));
            core.remove_editor_layer(id, &reseed);
        }

        if ctx.active_layer.get_untracked().as_deref() == Some(id) {
            ctx.active_layer.set(None);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Reparent a folder (drag-in-tree / root-dropzone). Rides the cycle-guarded `reparent_editor_layer` (a drop into the folder's own subtree is a no-op at the core), so this wrapper does not re-check cycles. `new_parent = None` moves it to the root. One transaction ⇒ one undo step.
pub fn reparent_layer(id: &str, new_parent: Option<String>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.reparent_editor_layer(id, new_parent);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Refile a slot into a different folder (drag a slot row onto a folder). Rides the shipped `move_slot_to_layer` (detach from every folder holding it, append to the target); squad is unchanged (workflow-only). One transaction ⇒ one undo step.
pub fn refile_slot_to_layer(slot_id: &str, layer_id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_layer(slot_id, layer_id);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
