//! Role: undo, redo, and the post-change tail every committed edit runs.
//! Position: `editing/history` in the map engine.
//! Signals & state: the hosted document's own undo stack.
//! Invariants: the mutable borrow a step needs is scoped and dropped BEFORE the host's tail runs,
//! because that tail opens read borrows of the same document. A step that changed nothing runs no
//! tail, so a button fired against an empty stack costs a host nothing.

use crate::data::store::MissionDocCore;
use crate::editing::host::with_doc_mut;

use super::host::host;

/// Undo the last LOCAL transaction; `true` if anything was undone. A no-op on an empty stack, so
/// callers may fire it unconditionally.
pub fn undo() -> bool {
    step(MissionDocCore::undo)
}

/// Redo the last undone transaction; `true` if anything was redone.
pub fn redo() -> bool {
    step(MissionDocCore::redo)
}

/// Run the post-change tail after a mutator the caller already committed. The same tail undo and
/// redo take.
pub fn after_local_edit() {
    (host().after_document_change)();
}

/// One step of the document's stack, then the tail — with the mutable borrow dropped in between.
fn step(f: impl FnOnce(&mut MissionDocCore) -> bool) -> bool {
    let did = with_doc_mut(f).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}
