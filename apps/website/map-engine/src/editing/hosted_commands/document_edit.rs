//! Role: run one mutator against the hosted document and take the post-change tail.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: the document borrow is dropped before the tail runs, because that tail opens read
//! borrows of the same document. An edit made with no document hosted runs no tail and reports
//! `false`, which every caller reads as "nothing happened" rather than as a failure.

use crate::data::store::MissionDocCore;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

/// Run `edit` against the live document, then the tail, so the whole edit is one undo step.
/// `false` when no document is hosted.
pub fn commit_document_edit(edit: impl FnOnce(&MissionDocCore)) -> bool {
    let did = with_doc(edit).is_some();
    if did {
        after_local_edit();
    }
    did
}
