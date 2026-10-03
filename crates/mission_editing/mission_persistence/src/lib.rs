//! The decidable half of the Mission Creator's local drafts.
//!
//! **Role:** decides what a draft record is worth, which key it belongs under, how a record already
//! on disk is reconciled with the one about to replace it, and, when a saved server version and a
//! local draft both claim to be the mission, which of the two wins and what is kept before the
//! loser is overwritten.
//! **Position:** mission editing category, tier 7, over `mission_editing_session` (the shared
//! document handle), `mission_document`, `mission_model` (the mission id) and `mission_payload`
//! (the compile the server comparison runs). The Mission Creator's draft shell calls it through
//! the map engine's `editing::persist` path and owns the storage and the network.
//! **Signals & state:** none of its own; every item is pure over its arguments, or over a document
//! handle and closures the host passes in.
//! **Invariants:** where the bytes live and how they travel is the host's, and no storage or
//! network API is named here, so every decision is answerable by `cargo test` with no browser.

/// The physical key one account's draft record lives under, and how to read one back.
pub mod record_key;

/// Whether a mission id names a row the server can hold.
pub mod mission_id;

/// Whether a stored blob restores to a document that holds authored content.
pub mod stored_blob;

/// How long to wait before re-reading a record whose read failed.
pub mod record_read_retry;

/// Reconcile the record already on disk with the one about to replace it.
pub mod merge_policy;

/// A canonical, order-independent fingerprint of a document's materialized slots.
pub mod slot_fingerprint;

/// Whether the local draft and the server's current version are the same document.
pub mod local_versus_server;

/// Replace the document with a server payload, and stamp the mission row onto it.
pub mod server_adoption;

/// The two slots a whole-document snapshot lives in, and the capture that fills one.
pub mod snapshot_slot;

/// The names most readers import.
pub mod prelude;
