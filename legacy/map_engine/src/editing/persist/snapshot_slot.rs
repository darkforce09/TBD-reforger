//! Role: the two slots a whole-document snapshot lives in, and the capture that fills one.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none of its own; the document arrives as a handle and where the bytes go is the
//! host's closure.
//! Invariants: the two slots are a **pair, not a stack**. Every swap writes the document it
//! displaces into the other slot, so a restore and its inverse are exact opposites: the door swings
//! both ways however many times it is pushed, and neither record is ever consumed by reading it. A
//! capture encodes BEFORE anything is replaced, so the bytes are pre-swap by construction; an empty
//! encode is refused, because replacing a good record with an empty one is the loss the snapshot
//! exists to prevent.

use crate::editing::host::DocHandle;

/// Which destructive whole-document replacement a snapshot is the escape hatch from.
///
/// Both records live beside the live draft under a **suffixed** key, which is the whole point: the
/// debounced editor write re-arms on every swap and rewrites the plain mission id moments later,
/// and that write must not be able to reach either record. A mission id is a canonical UUID, so
/// neither suffix can collide with one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotSlot {
    /// The local work, captured before an adopt replaces it with the server's payload.
    ///
    /// Local work exists nowhere else, so this is the record that actually matters — it is dropped
    /// only by an explicit save, never to make room.
    PreAdopt,

    /// The adopted server document, captured before a restore replaces it with [`Self::PreAdopt`].
    ///
    /// Cheap to lose relative to its counterpart — a server version is always one refetch away —
    /// which is why this is the slot a new conflict is allowed to invalidate.
    PreRestore,
}

impl SnapshotSlot {
    /// The record-key suffix. Distinct literals rather than a name derived from the variant: these
    /// strings are the on-disk contract for records that are read back after a reload.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::PreAdopt => "::pre-adopt",
            Self::PreRestore => "::pre-restore",
        }
    }

    /// Human-readable name for a refusal message or a warning.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::PreAdopt => "pre-adopt",
            Self::PreRestore => "pre-restore",
        }
    }

    /// The slot a restore of `self` must write its displaced document into — the source slot of the
    /// inverse verb.
    #[must_use]
    pub fn counterpart(self) -> Self {
        match self {
            Self::PreAdopt => Self::PreRestore,
            Self::PreRestore => Self::PreAdopt,
        }
    }
}

/// Capture the whole live document and hand the bytes to `store_snapshot_bytes`. Returns how many
/// slots were captured, or `None` when there was nothing worth storing.
///
/// The encode is the same update stream a warm reopen replays, so a snapshot is restorable by
/// exactly the path the editor already proves on every reload — no second serialization format and
/// no second trust.
///
/// The ORDER is what belongs here and why the store arrives as a closure. The encode happens inside
/// this call, synchronously, before the caller has replaced anything, so "these bytes are the
/// pre-swap document" is true by construction rather than by a caller remembering to encode first.
/// An empty encode returns `None` without calling the store at all: an empty blob can only replace
/// a good record with a bad one.
pub fn capture_document_snapshot(
    doc: &DocHandle,
    store_snapshot_bytes: &dyn Fn(Vec<u8>),
) -> Option<usize> {
    let (bytes, slots) = {
        let guard = doc.borrow();
        let core = guard.as_ref()?;
        (core.encode_state(), core.slot_count())
    };
    if bytes.is_empty() {
        return None;
    }
    store_snapshot_bytes(bytes);
    Some(slots)
}

#[cfg(test)]
#[path = "tests/snapshot_slot.rs"]
mod tests;
