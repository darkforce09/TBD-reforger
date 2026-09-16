//! Role: the editor's selected entities — replacing the set, selecting a folder's rows, selecting
//! everything in view, and framing the selection with the camera.
//! Position: `editor/state` in the frontend editor shell.
//! Signals & state: the selected-id set on the installed editor context, the render engine handle
//! whose tint and camera it drives, and the dock mirrors the shared refresh pushes.
//! Invariants: one write of the set, then the renderer, then the mirrors — in that order, from one
//! place, so a tint and a dock row can never disagree about what is selected. Selection is never
//! document state: nothing here mints an undo step, and a selection over ids the document no longer
//! holds is pruned by the post-change tail rather than defended against here.

use crate::editor::state::history as mission_history;
use crate::editor::state::operations::context::OPS_CTX;
use website_map_engine::data::store::operations::projections::layer_rows;
use website_map_engine::editing::hosted_commands::vehicle_points;
use website_map_engine::editing::tools::selection;

/// Replace the selected ids, rebind the renderer's tint, and refresh the mirrors.
pub fn set_slot_selection(ids: Vec<String>) {
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

/// Replace the selection with an explicit id list. An empty list is refused rather than treated as
/// "clear" — clearing is its own gesture, and a caller handing over nothing has usually resolved
/// nothing. Returns how many ids were selected.
pub fn set_selection_ids(ids: Vec<String>) -> usize {
    if ids.is_empty() {
        return 0;
    }
    let n = ids.len();
    set_slot_selection(ids);
    n
}

/// Replace the selection with `id`, unless `id` is already part of a multi-selection — in which
/// case the click is a grab on a set the operator already made, and collapsing it to one row would
/// throw that set away before the drag they were starting could use it.
pub fn select_slot(id: String) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };

        let keep_multi = {
            let sel = ctx.selection.borrow();
            sel.len() > 1 && sel.iter().any(|s| *s == id)
        };
        if !keep_multi {
            *ctx.selection.borrow_mut() = vec![id];
            let ids = ctx.selection.borrow().clone();

            let mut eng = ctx.engine.borrow_mut();
            if let Some(e) = eng.as_mut() {
                e.set_selection(ids);
            }
        }
    });
    mission_history::refresh_selection();
}

/// Select a folder's DIRECT slot children, replacing the selection.
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

/// Select every slot in a folder's whole subtree, replacing the selection.
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

/// Select every slot and vehicle inside the current view. The vehicle points are resolved BEFORE
/// the context borrow opens, because that read opens its own. Returns whether it acted, so the
/// keydown arm can `prevent_default` — the browser's own Select All would otherwise blue-wash the
/// editor chrome.
pub fn select_all_in_view(viewport_w: f64, viewport_h: f64) -> bool {
    if !(viewport_w > 0.0 && viewport_h > 0.0) {
        return false;
    }
    let points = vehicle_points();
    let acted = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let cam = {
            let eng = ctx.engine.borrow();
            let Some(e) = eng.as_ref() else {
                return false;
            };
            selection::frozen_camera(viewport_w, viewport_h, e.target_x(), e.target_y(), e.zoom())
        };
        let ids = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            selection::view_ids_with_vehicles(&cam, &core.materialize(), &points)
        };

        let slot_ids: Vec<String> = ids
            .iter()
            .filter(|i| !points.iter().any(|(v, _, _)| v == *i))
            .cloned()
            .collect();
        *ctx.selection.borrow_mut() = ids;
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(slot_ids);
        }
        true
    });
    if acted {
        mission_history::refresh_selection();
    }
    acted
}

/// Move the camera to the selection's centroid, keeping the zoom. `false` when nothing is selected
/// or the selection has no position to average.
pub fn center_on_selection() -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Some((sx, sy)) =
            website_map_engine::data::store::operations::entity::selection_centroid(core, &sel)
        else {
            return false;
        };
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_view(sx, sy, e.zoom());
            e.on_camera_changed();
            true
        } else {
            false
        }
    })
}
