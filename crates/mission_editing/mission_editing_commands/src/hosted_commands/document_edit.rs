//! **Role:** run one mutator against the hosted document and take the post-change tail.
//! **Position:** `hosted_commands::document_edit` in `mission_editing_commands`.
//! **Signals & state:** none of its own; the document comes from the host.
//! **Invariants:** the document borrow is dropped before the tail runs, because that tail opens
//! read borrows of the same document. An edit made with no document hosted runs no tail and reports
//! `false`, which every caller reads as "nothing happened" rather than as a failure.

use mission_document::MissionDocCore;
use mission_editing_session::history::after_local_edit;
use mission_editing_session::host::with_doc;

/// Run `edit` against the live document, then the tail, so the whole edit is one undo step.
/// `false` when no document is hosted.
pub fn commit_document_edit(edit: impl FnOnce(&MissionDocCore)) -> bool {
    let did = with_doc(edit).is_some();
    if did {
        after_local_edit();
    }
    did
}
