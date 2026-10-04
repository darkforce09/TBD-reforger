//! The items most callers of the engine bridge name.
//!
//! **Role:** one import for the undo drive, the editor context's attribute and environment
//! accessors, the hosted document handle and the canvas gesture context.
//! **Position:** re-exports of this crate's own modules; nothing is defined here.
//! **Signals & state:** none.
//! **Invariants:** re-exports only this crate's items, with the same `wasm32` gates as their
//! definitions.

#[cfg(target_arch = "wasm32")]
pub use crate::bridge::document_host::doc_host::DocHandle;
#[cfg(target_arch = "wasm32")]
pub use crate::bridge::document_host::history::{doc_handle, is_dirty, redo, set_dirty, undo};
#[cfg(target_arch = "wasm32")]
pub use crate::input::pointer_gestures::EditorGestureContext;
