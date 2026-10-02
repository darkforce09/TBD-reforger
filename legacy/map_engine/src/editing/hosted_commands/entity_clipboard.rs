//! Role: cut, copy and paste over the hosted document's selection.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: the copied rows, one buffer per thread. The document, the selected ids and the
//! id minter come from the host.
//! Invariants: the buffer holds authored slot rows, not ids, so a paste still works after its
//! sources are deleted or undone. A copy mutates nothing and runs no tail; a delete and a paste
//! each run exactly one, after the document borrow has been dropped. The layer a paste files into
//! is the HOST's answer and crosses as a closure, because which folder is active is host state.

use std::cell::RefCell;
use std::collections::HashSet;

use crate::data::store::MissionDocCore;
use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, set_selection_ids, with_doc, with_host};

thread_local! {
    static CLIPBOARD: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}

/// Delete every selected entity — comments, the connections that touch them, and the slots — and
/// leave nothing selected. `false` when nothing is selected or no document is hosted.
///
/// Which selected ids are comments is asked of the comment rows themselves, the same read the
/// Outliner, the map lane and the map pick are built from — never of a `cmt-` prefix on the id.
/// That prefix is the minter's convention and not a document invariant, and a hydrated mission is
/// free to carry comment ids that were never minted here.
pub fn delete_selection() -> bool {
    let ids = selection_ids();
    if ids.is_empty() {
        return false;
    }
    let removed = with_doc(|core| entity_ops::delete_selection(core, ids)).is_some();
    if removed {
        set_selection_ids(Vec::new());
        after_local_edit();
    }
    removed
}

/// Copy the selected slots into the buffer. `false` when nothing is selected or the selection
/// captured no slot row, and in both cases the previous buffer is kept — an empty copy must not
/// silently destroy what the operator still means to paste.
pub fn copy_selection() -> bool {
    let sel: HashSet<String> = selection_ids().into_iter().collect();
    if sel.is_empty() {
        return false;
    }
    let Some(clip) = with_doc(|core| entity_ops::copy_selection(core, &sel)).flatten() else {
        return false;
    };
    CLIPBOARD.with(|cb| *cb.borrow_mut() = clip);
    true
}

/// Paste the buffer at `(cx, cy)`, or at the buffer's own offset when no cursor is given, and
/// select what was placed. `ensure_layer` is the host's answer to "which folder does a new entity
/// go in", called against the live document so a minted default folder is part of the same edit.
pub fn paste_at_cursor(
    cx: Option<f64>,
    cy: Option<f64>,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> bool {
    let clip = CLIPBOARD.with(|cb| cb.borrow().clone());
    if clip.is_empty() {
        return false;
    }
    let placed = with_host(|host| {
        let doc = host.doc.borrow();
        let Some(core) = doc.as_ref() else {
            return Vec::new();
        };
        let layer_id = ensure_layer(core);
        let ids = entity_ops::paste_at_cursor(core, clip, layer_id, &host.next_id, cx, cy);
        *host.selection.borrow_mut() = ids.clone();
        ids
    })
    .unwrap_or_default();
    if placed.is_empty() {
        return false;
    }
    after_local_edit();
    true
}
