//! The viewer state machine and the read outcomes it lands.
//!
//! **Role:** re-exports `ViewerState`, `LoadedDocument` and `DocumentOutcome`.
//! **Position:** built by `crate::document_viewer::services::document_loading`, which re-exports the
//! types; held by the desktop application and painted by its document column.
//! **Signals & state:** `ViewerState` is the column's state (closed, loading, rendered, fallback).
//! **Invariants:** a read lands only while the viewer still waits for that same path.

mod read_outcome;
mod viewer_state;
pub use read_outcome::*;
pub use viewer_state::*;
