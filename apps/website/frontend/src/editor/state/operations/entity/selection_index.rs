//! Role: selection index.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;
use website_map_engine::editing::hosted_commands::vehicle_points;
use website_map_engine::editing::tools::selection;

/// Derived from the document index rather than re-read per kind, so the selection filter's chips and the search's rows can never disagree about what an entity's type or faction is. Selection order is not preserved (the document order is): the chips are counts and id sets, and nothing downstream reads a selection as a sequence.
#[must_use]
pub fn selection_entities() -> Vec<crate::editor::panels::dock_left::DocEntity> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let sel = ctx.selection.borrow().clone();
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            Some(
                website_map_engine::data::store::operations::entity::selection_entities(core, &sel),
            )
        })
        .unwrap_or_default()
}

/// Set selection ids using the supplied domain data.
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

/// Select every slot and vehicle inside the current view. `vehicle_points()` is resolved BEFORE the
/// context borrow opens, because it opens its own. Returns whether it acted, so the keydown arm can
/// `prevent_default` — the browser's own Select All would otherwise blue-wash the editor chrome.
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
