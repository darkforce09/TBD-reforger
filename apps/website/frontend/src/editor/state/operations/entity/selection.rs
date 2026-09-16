//! Role: selection.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Read searchable entities from the current document.
pub fn document_entities() -> Vec<crate::editor::panels::dock_left::DocEntity> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            Some(
                website_map_engine::data::store::operations::document_index::document_entities(
                    core,
                ),
            )
        })
        .unwrap_or_default()
}

/// Read the placed owners from the current document.
pub fn placed_owner_options() -> Vec<OwnerOption> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            Some(website_map_engine::data::store::operations::entity::placed_owner_options(core))
        })
        .unwrap_or_default()
}

/// Canonical default layer id value.
pub(in crate::editor::state::operations) const DEFAULT_LAYER_ID: &str = "layer-1";

/// Canonical default layer name value.
pub(in crate::editor::state::operations) const DEFAULT_LAYER_NAME: &str = "Layer 1";

/// Read comment using the supplied domain data.
#[must_use]
pub fn read_comment(id: &str) -> Option<CommentDetail> {
    comment_list().into_iter().find(|c| c.id == id)
}

/// The membership question is asked of [`comment_details`], the same `comments_json` read the Outliner rows, the map lane and the map pick are built from — not a prefix test on the id. A `cmt-` prefix is [`mint_comment_id`]'s convention, not a document invariant, and a hydrated mission is free to carry comment ids that were never minted here.
pub fn delete_selection() -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let ids = ctx.selection.borrow().clone();
        if ids.is_empty() {
            return false;
        }
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            website_map_engine::data::store::operations::entity::delete_selection(core, ids)
        }
        ctx.selection.borrow_mut().clear();
        true
    });
    if removed {
        mission_history::after_local_edit();
    }
    removed
}

/// Center on selection using the supplied domain data.
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

/// Copy selection using the supplied domain data.
pub fn copy_selection() -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel: std::collections::HashSet<String> =
            ctx.selection.borrow().iter().cloned().collect();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Some(clip) =
            website_map_engine::data::store::operations::entity::copy_selection(core, &sel)
        else {
            return false;
        };
        CLIPBOARD.with(|cb| *cb.borrow_mut() = clip);
        true
    })
}

/// Paste at cursor using the supplied domain data.
pub fn paste_at_cursor(cx: Option<f64>, cy: Option<f64>) -> bool {
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let clip = CLIPBOARD.with(|cb| cb.borrow().clone());
        if clip.is_empty() {
            return Vec::new();
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let layer_id = ensure_layer(ctx, core);
        let ids = website_map_engine::data::store::operations::entity::paste_at_cursor(
            core,
            clip,
            layer_id,
            &ctx.next_id,
            cx,
            cy,
        );
        *ctx.selection.borrow_mut() = ids.clone();
        ids
    });
    if !placed.is_empty() {
        mission_history::after_local_edit();
        true
    } else {
        false
    }
}

/// `vehicle_points()` is resolved BEFORE the `OPS_CTX` borrow opens (it opens its own), keeping the module's one-borrow-per-`pub fn` discipline. Returns whether it acted, so the keydown arm can `prevent_default` — the browser's own Select All would otherwise blue-wash the editor chrome.
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
            crate::editor::tools::select_tool::frozen_camera(
                viewport_w,
                viewport_h,
                e.target_x(),
                e.target_y(),
                e.zoom(),
            )
        };
        let ids = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            crate::editor::tools::select_tool::view_ids_with_vehicles(
                &cam,
                &core.materialize(),
                &points,
            )
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

/// Select slot using the supplied domain data.
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
