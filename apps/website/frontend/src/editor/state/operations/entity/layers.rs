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

/// Run one layer edit against the live doc, then the shared dirty tail. `false` when the ops
/// context is not up or the handle holds no document.
pub(in crate::editor::state::operations) fn edit_layer(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Set layer hidden using the supplied domain data.
pub fn set_layer_hidden(id: &str, hidden: bool) {
    edit_layer(|core| {
        website_map_engine::data::store::operations::entity::set_layer_hidden(core, id, hidden);
    });
}

/// Set layer locked using the supplied domain data.
pub fn set_layer_locked(id: &str, locked: bool) {
    edit_layer(|core| {
        website_map_engine::data::store::operations::entity::set_layer_locked(core, id, locked);
    });
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
        website_map_engine::data::store::operations::entity::show_all_hidden(core)
    });
    if cleared > 0 {
        mission_history::after_local_edit();
    }
    cleared
}

/// LAYER-CREATE-001 — create a folder as a **child of the selected/active folder** (or a root when none is active), auto-named "New Layer N", and arm its inline rename. Returns the new id.
pub fn create_layer() -> Option<String> {
    let created = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let id = {
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            website_map_engine::data::store::operations::entity::create_layer(
                core,
                ctx.active_layer.get_untracked(),
            )
        };

        ctx.active_layer.set(Some(id.clone()));
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
    website_map_engine::data::store::operations::entity::take_rename_armed()
}

/// Rename an Outliner folder (inline-rename commit). Rides the shipped `rename_editor_layer`; one transaction ⇒ one undo step. A blank name after trim is rejected (a folder must keep a label).
pub fn rename_layer(id: &str, name: &str) -> bool {
    let renamed = OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            Some(website_map_engine::data::store::operations::entity::rename_layer(core, id, name))
        })
        .unwrap_or(false);
    if renamed {
        mission_history::after_local_edit();
    }
    renamed
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
            website_map_engine::data::store::operations::entity::delete_layer(core, id);
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
    edit_layer(|core| {
        website_map_engine::data::store::operations::entity::reparent_layer(core, id, new_parent);
    })
}

/// Refile a slot into a different folder (drag a slot row onto a folder). Rides the shipped `move_slot_to_layer` (detach from every folder holding it, append to the target); squad is unchanged (workflow-only). One transaction ⇒ one undo step.
pub fn refile_slot_to_layer(slot_id: &str, layer_id: &str) -> bool {
    edit_layer(|core| {
        website_map_engine::data::store::operations::entity::refile_slot_to_layer(
            core, slot_id, layer_id,
        );
    })
}
