//! Role: selection index.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Derived from the document index rather than re-read per kind, so the selection filter's chips and the search's rows can never disagree about what an entity's type or faction is. Selection order is not preserved (the document order is): the chips are counts and id sets, and nothing downstream reads a selection as a sequence.
#[must_use]
pub fn selection_entities() -> Vec<crate::editor::panels::dock_left::DocEntity> {
    let sel: Vec<String> = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().clone())
            .unwrap_or_default()
    });
    if sel.is_empty() {
        return Vec::new();
    }
    document_entities()
        .into_iter()
        .filter(|e| sel.iter().any(|id| id == &e.id))
        .collect()
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
