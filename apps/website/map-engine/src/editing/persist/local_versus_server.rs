//! Role: decide whether the local draft and the server's current version are the same document.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none; pure over the document it is handed and the payload it is compared to.
//! Invariants: the question is **would adopting this payload change the document**, and it is
//! answered by comparing the two documents rather than by comparing a version number. A version
//! marker is evidence about a *number* standing in for evidence about *content*, and it is wrong in
//! both directions: a missing marker asks the operator to choose between two identical documents,
//! and a matching one vouches for local work it has never seen. Only a difference that actually
//! exists may raise a prompt, and every difference that exists must raise one.

use crate::data::scenario::compile::compile_payload;
use crate::data::store::MissionDocCore;
use crate::editing::host::DocHandle;
use crate::editing::persist::server_adoption::DEFAULT_LAYER_ID;

/// What the local draft holds, measured against the server's current version.
///
/// Three states and not two. "Local exists" is not the same question as "local might be lost": most
/// of the time the local draft IS the server's document, and there is nothing at stake to ask
/// about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LocalDraftVerdict {
    /// The document holds nothing authored — a local record exists but decodes to an empty
    /// document. There is no local work, so there is no choice to offer.
    Empty,

    /// Adopting the server payload would reproduce this document exactly. The adopt is a no-op, so
    /// nothing is at stake and nothing is asked.
    MatchesServer,

    /// The two documents differ. The only state that warrants a prompt.
    Diverged,
}

/// The compiled keys that carry **authored** content — the whole of the comparison.
///
/// Deliberately excluded: the map (terrain and its derived bounds) and the environment (time and
/// weather). Those are mission-ROW fields, supplied separately and written into the local document
/// on every boot *after* the hydrate — so the local document is expected to hold the row's current
/// values while a saved payload holds whatever they were when it was saved. Comparing them would
/// turn "somebody changed the weather dropdown" into a data-loss prompt. The schema version is a
/// constant, and the order of battle is an export-only projection that neither compile emits here.
const AUTHORED_KEYS: [&str; 5] = ["editor", "loadouts", "objectives", "vehicles", "markers"];

/// How many slots the server payload carries. Absent or malformed reads as none, which is exactly
/// what a hydrate of that payload would produce.
#[must_use]
pub fn server_slot_count(server: &serde_json::Value) -> usize {
    server
        .pointer("/editor/slots")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// Do two compiled payloads carry the same authored document?
///
/// Deep value equality over the authored keys. Object equality ignores the order the keys were
/// written in, and both sides are compiled out of a document by the same code — so this compares
/// what the two documents hold, not the bytes either payload happened to be spelled with.
#[must_use]
pub fn same_authored_content(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    AUTHORED_KEYS.iter().all(|k| a.get(*k) == b.get(*k))
}

/// Classify the live document against the server's current version. `server` is the payload as
/// fetched; `payload_json` is that same payload serialized — the exact bytes an adopt would
/// hydrate, so the comparison is against the document the adopt would actually produce.
///
/// `None` means there is no document to classify at all (the editor went away mid-boot), which is
/// distinct from [`LocalDraftVerdict::Empty`] — a real document that happens to hold nothing.
///
/// Three tiers, cheapest first, because this runs on every warm open of every saved mission and the
/// last tier costs a pass over the whole document:
///   1. "does the document hold anything authored" — constant time.
///   2. slot count against the payload's slot count — constant time on both sides, and it settles
///      the common divergence (something was placed or deleted) without serializing anything. This
///      is what keeps a very large mission off the deep tier unless it genuinely might be identical.
///   3. compile both documents and compare the authored keys.
#[must_use]
pub fn classify_local_draft(
    doc: &DocHandle,
    server: &serde_json::Value,
    payload_json: &str,
) -> Option<LocalDraftVerdict> {
    let guard = doc.borrow();
    let core = guard.as_ref()?;
    if !core.has_content() {
        return Some(LocalDraftVerdict::Empty);
    }
    if core.slot_count() != server_slot_count(server) {
        return Some(LocalDraftVerdict::Diverged);
    }
    // Compare against the document the adopt WOULD produce, built by running the adopt's own
    // hydrate on a throwaway document. Both sides then reach the compiler by the identical path, so
    // nothing a hydrate would normalize away can register as a difference: the default-layer
    // reseed, the roots a hydrate clears and never loads, and the CRDT's integer encoding all line
    // up by construction rather than by a rule restated here and left to drift. Authored ROW ORDER
    // does take part, because the compiler emits rows in the document's own order — two documents
    // holding the same rows in a different order are two different documents here.
    //
    // Initialization origin for the same reason every other throwaway replay uses it: a local apply
    // pushes an undo step and the CRDT keeps in-scope deleted blocks alive for as long as the stack
    // item lives. This document is dropped at the end of the function and should cost one document,
    // not two.
    let offered = MissionDocCore::new();
    offered.set_origin_init(true);
    offered.hydrate(payload_json, DEFAULT_LAYER_ID);
    offered.set_origin_init(false);
    let mine = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
    let theirs = compile_payload(&offered.small_maps_json(), &offered.slots_json(), false);
    Some(if same_authored_content(&mine, &theirs) {
        LocalDraftVerdict::MatchesServer
    } else {
        LocalDraftVerdict::Diverged
    })
}

#[cfg(test)]
#[path = "tests/local_versus_server.rs"]
mod tests;
