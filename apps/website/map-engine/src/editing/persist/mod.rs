//! Role: Module boundary for the decidable half of local draft persistence.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none of its own — every item here is pure over its arguments, or over a
//! document handle and closures the host passes in.
//! Invariants: this module decides WHAT a draft record is worth, which key it belongs under, how a
//! record already on disk is reconciled with the one about to replace it, and — when a saved server
//! version and a local draft both claim to be the mission — which of the two wins and what is kept
//! before the loser is overwritten. WHERE the bytes live and how they travel is the host's, and no
//! storage API is named below — which is what makes every decision here answerable by `cargo test`
//! with no browser.

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
