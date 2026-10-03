//! The Mission Creator's editing session over the live mission document.
//!
//! **Role:** holds the installed editing host and the one borrow chain every editing command
//! reaches the document by ([`host`]), steps the document's own undo stack and runs the host's
//! post-change tail ([`history`]), groups several transactions into one undo step ([`batch`]),
//! resolves a subject id to the selection surface that owns it ([`routing`]), answers which ids a
//! selection may name and derives the map-render slot view ([`selection_universe`]), joins spatial
//! picks to document ids ([`picking`]) and draws the editor-authored overlay lanes ([`lanes`]).
//! **Position:** mission editing category, tier 6, over `mission_document`, `mission_crdt`,
//! `mission_validation`, `camera_math`, `spatial_indexes` and `unit_symbology`. The hosted
//! commands, the map tools and the draft persistence build on it; the Mission Creator installs the
//! host and calls it through the map engine's `editing` module.
//! **Signals & state:** one thread-local editing host (the document handle, the selected ids and
//! the id minter) and one thread-local undo-drive service table; everything else is pure over its
//! arguments.
//! **Invariants:** no browser, UI framework or GPU type crosses into this crate; every entry point
//! opens one document borrow and drops it before returning; the undo drive's mutable borrow ends
//! before the host's tail runs.

pub mod batch;
pub mod history;
pub mod host;
pub mod lanes;
pub mod picking;
pub mod prelude;
pub mod routing;
pub mod selection_universe;
