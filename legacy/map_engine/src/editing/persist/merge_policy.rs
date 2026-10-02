//! Role: reconcile the draft record already on disk with the one about to replace it.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none of its own; the document to merge into, the record to read, and the
//! encode of the live document all arrive from the caller.
//! Invariants: a merge is a CRDT union (`MissionDocCore::apply_update`) and never a field-wise
//! diff, so two authors who both hold the same key keep both halves of their work. A record that
//! belongs to a different mission is never applied. After a merge the document IS the union, so
//! the bytes that go out are re-encoded — writing the pre-merge blob would make the read
//! decoration.

use core::future::Future;

use crate::editing::host::DocHandle;

/// Apply a stored record into the live document, and report whether it landed.
///
/// **This is the whole of the merge, and it is `MissionDocCore::apply_update` — a CRDT union.** Two
/// documents restored from one record author under different client ids, so a sibling's blocks
/// integrate rather than collide; replaying a document's own earlier blob is discarded as
/// already-seen, which makes the replay harmless but pointless, and is why a caller that can prove
/// the record is its own skips this path entirely.
///
/// `apply_update` transacts under the initialization origin regardless of the core's mode, so a
/// merge is **not** an undo step: an undo after a peer's edits arrive undoes the operator's own
/// last action, not the sync.
///
/// `record_mission_id` is checked against `document_mission_id` rather than assumed. A pending
/// write can outlive a route change, so "the document the editor has open" is not by itself an
/// answer to "the mission these bytes came from", and applying one mission's blocks into another's
/// document is unrecoverable.
///
/// `false` means nothing was applied: a different mission, no document in the handle, or a blob the
/// CRDT refused. Nothing is mutated in any of those cases.
#[must_use]
pub fn apply_update_into_document(
    doc: &DocHandle,
    document_mission_id: &str,
    record_mission_id: &str,
    stored: &[u8],
) -> bool {
    if document_mission_id != record_mission_id {
        return false;
    }
    let guard = doc.borrow();
    let Some(core) = guard.as_ref() else {
        return false;
    };
    core.apply_update(stored).is_ok()
}

/// Read the record this write is about to land on, merge it into the live document, and hand back
/// the bytes to write.
///
/// Returns `bytes` unchanged when there is nothing to merge — no record, an unreadable one, a
/// record byte-identical to what is about to be written, or a blob the CRDT refuses. Otherwise it
/// **re-encodes**, because after the merge the document is the union and the pre-merge encode is
/// no longer what it holds. That re-encode is the difference between "we looked" and "we merged".
///
/// The read, the merge and the encode all arrive as closures, and deliberately so: the record
/// store, the document registry and the host's encode are the host's business. What belongs here
/// is the ORDER. The read happens inside this call rather than before it, because every moment
/// between reading a record and writing over it is a moment another writer can make the answer
/// wrong, and reading late shrinks that window to this function's own body.
pub async fn merge_before_write<ReadStoredRecord, StoredRecordFuture>(
    bytes: Vec<u8>,
    read_stored_record: ReadStoredRecord,
    merge_into_document: &dyn Fn(&[u8]) -> bool,
    encode_document: &dyn Fn() -> Vec<u8>,
) -> Vec<u8>
where
    ReadStoredRecord: FnOnce() -> StoredRecordFuture,
    StoredRecordFuture: Future<Output = Option<Vec<u8>>>,
{
    let Some(stored) = read_stored_record().await else {
        return bytes;
    };
    if stored == bytes || !merge_into_document(&stored) {
        return bytes;
    }
    encode_document()
}

#[cfg(test)]
#[path = "tests/merge_policy.rs"]
mod tests;
