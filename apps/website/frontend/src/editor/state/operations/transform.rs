//! Role: transform.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use crate::editor::state::history as mission_history;

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, compositions::*, context::*, entity::*};

/// Returns whether anything rotated (nothing selected, or every entity sitting exactly under the cursor, is a no-op — [`website_map_engine::data::store::operations::rotation::bearing_to_face`] returns `None` for a degenerate aim and that entity is left untouched).
pub fn rotate_selection_to_face(cx: f64, cy: f64, rung: usize) -> bool {
    if !cx.is_finite() || !cy.is_finite() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
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
        website_map_engine::data::store::operations::transform::rotate_selection_to_face(
            core, cx, cy, rung, sel,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

#[cfg(target_arch = "wasm32")]
fn confirm_bulk(n: usize, verb: &str) -> bool {
    if !crate::editor::tools::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue? (Ctrl+Z undoes the whole op.)");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Confirm for bulk ops that are still N undo steps (loadout apply/remove — no atomic batch yet).
#[cfg(target_arch = "wasm32")]
pub(super) fn confirm_bulk_n_step(n: usize, verb: &str) -> bool {
    if !crate::editor::tools::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue?");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Apply pattern to selection using the supplied domain data.
pub fn apply_pattern_to_selection(kind: crate::editor::tools::place_helpers::PatternKind) -> bool {
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
        website_map_engine::data::store::operations::transform::apply_pattern_to_selection(
            core,
            kind,
            sel,
            confirm_bulk,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Align selection using the supplied domain data.
pub fn align_selection(edge: crate::editor::tools::place_helpers::AlignEdge) -> bool {
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
        website_map_engine::data::store::operations::transform::align_selection(
            core,
            edge,
            sel,
            confirm_bulk,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Space selection using the supplied domain data.
pub fn space_selection(axis: crate::editor::tools::place_helpers::SpaceAxis) -> bool {
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
        website_map_engine::data::store::operations::transform::space_selection(
            core,
            axis,
            sel,
            confirm_bulk,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Orient selection using the supplied domain data.
pub fn orient_selection(cmd: crate::editor::tools::place_helpers::Orient) -> bool {
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
        website_map_engine::data::store::operations::transform::orient_selection(
            core,
            cmd,
            sel,
            confirm_bulk,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
