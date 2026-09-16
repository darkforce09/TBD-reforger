//! Role: decide whether a stored blob is worth keeping — and worth writing.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none; pure over bytes.
//! Invariants: "would restoring this produce a document with authored content" is answered by
//! DOING the restore, never by a byte length. An empty document encodes to two non-empty bytes, so
//! every threshold on length is a guess and this is not. A blob that cannot be replayed is not a
//! backup, and answers `false` for the same reason an empty one does.

use crate::data::store::MissionDocCore;

/// Read the leading unsigned var-int of a Yjs v1 update stream: the **number of client blocks**.
///
/// `encode_state_as_update_v1` writes `varint(num_clients)`, then that many per-client struct
/// blocks, then the delete set. `num_clients == 0` therefore means the stream carries no structs at
/// all — a document with literally nothing in it, not even its metadata map. `None` means the
/// leading var-int is malformed (an unterminated continuation run), which is itself grounds to
/// refuse the blob.
///
/// This is the O(1) tier in front of [`restores_to_authored_content`], so the two bytes an empty
/// document encodes to are rejected without decoding anything at all.
fn leading_client_block_count(bytes: &[u8]) -> Option<u64> {
    let mut n: u64 = 0;
    let mut shift = 0u32;
    for (i, b) in bytes.iter().enumerate() {
        // A u64 var-int is at most 10 bytes; past that the stream is not a v1 update header.
        if i >= 10 {
            return None;
        }
        n |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Some(n);
        }
        shift += 7;
    }
    None
}

/// Would this blob restore to a document that holds authored content?
///
/// # Why this decodes rather than inspecting bytes
///
/// The question that matters is "would restoring these bytes produce a document with content", and
/// the sound way to answer it is to *do the restore* — the identical `MissionDocCore::new()` +
/// `apply_update` a boot restore runs — and then ask [`MissionDocCore::has_content`], the predicate
/// that already defines what content means here (faction / slot / objective / vehicle / marker).
///
/// Two classes of loss a byte-level test cannot see, and this one does:
///   * **content-empty but byte-fat.** A core with only its metadata seeded encodes to ~124 bytes
///     and holds no content. Any threshold on length is a guess; this is not.
///   * **corrupt or truncated.** A blob that fails `apply_update` is unrestorable, so writing it
///     over a good record trades a backup for nothing.
///
/// The decode is O(document). A caller that holds the live core the bytes were encoded from should
/// ask `has_content()` directly instead — O(1), and sound whenever the encode and the question are
/// sampled in the same synchronous window. This is the answer for a caller that has only bytes,
/// and it is the stronger of the two: it is the one that catches a corrupt blob.
#[must_use]
pub fn restores_to_authored_content(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    match leading_client_block_count(bytes) {
        // No client blocks => no structs => nothing in the document.
        Some(0) | None => return false,
        Some(_) => {}
    }
    let probe = MissionDocCore::new();
    // INIT, for the reason every other replay in this codebase uses it: a LOCAL apply pushes an
    // undo step and the CRDT keeps deleted blocks alive for as long as the stack item lives. This
    // core is dropped at the end of the function and should cost one document, not two.
    probe.set_origin_init(true);
    if probe.apply_update(bytes).is_err() {
        return false;
    }
    probe.set_origin_init(false);
    probe.has_content()
}

#[cfg(test)]
#[path = "tests/stored_blob.rs"]
mod tests;
