//! Role: align, distribute, re-orient and pattern the hosted selection, and rotate it to face a
//! point.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document and the selected ids come from the host.
//! Invariants: the confirmation a bulk move needs is the HOST's — every entry point here takes it
//! as a closure and the document layer calls it before committing, so the engine never owns a
//! prompt. A commit that moved nothing runs no post-change tail.

use crate::data::store::operations::transform;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, with_doc};
use crate::editing::tools::placement::{AlignEdge, Orient, PatternKind, SpaceAxis};

/// Rotate every selected entity to face `(cx, cy)`, quantised to `rung`. Returns whether anything
/// rotated: nothing selected, or an entity sitting exactly under the cursor, is a no-op — the
/// bearing is `None` for a degenerate aim and that entity is left untouched.
pub fn rotate_selection_to_face(cx: f64, cy: f64, rung: usize) -> bool {
    if !cx.is_finite() || !cy.is_finite() {
        return false;
    }
    let sel = selection_ids();
    if sel.is_empty() {
        return false;
    }
    let did = with_doc(|core| transform::rotate_selection_to_face(core, cx, cy, rung, sel))
        .unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Lay the selection out in `kind`'s pattern, asking `confirm_bulk` first when the move is large
/// enough to be destructive.
pub fn apply_pattern_to_selection(
    kind: PatternKind,
    confirm_bulk: impl Fn(usize, &str) -> bool,
) -> bool {
    let sel = selection_ids();
    let did = with_doc(|core| transform::apply_pattern_to_selection(core, kind, sel, confirm_bulk))
        .unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Align the selection to `edge`.
pub fn align_selection(edge: AlignEdge, confirm_bulk: impl Fn(usize, &str) -> bool) -> bool {
    let sel = selection_ids();
    let did =
        with_doc(|core| transform::align_selection(core, edge, sel, confirm_bulk)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Distribute the selection evenly along `axis`.
pub fn space_selection(axis: SpaceAxis, confirm_bulk: impl Fn(usize, &str) -> bool) -> bool {
    let sel = selection_ids();
    let did =
        with_doc(|core| transform::space_selection(core, axis, sel, confirm_bulk)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Re-orient the selection about its own centroid.
pub fn orient_selection(cmd: Orient, confirm_bulk: impl Fn(usize, &str) -> bool) -> bool {
    let sel = selection_ids();
    let did =
        with_doc(|core| transform::orient_selection(core, cmd, sel, confirm_bulk)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}
