//! The hosted mission document and the undo drive that moves it.
//!
//! **Role:** owns the `MissionDocCore` the open mission lives in — its lifecycle and the read-only
//! smoke bridge over it — and the app-side driver for the undo stack the core itself keeps, with
//! the post-change rebind that puts the renderer back in step with the document.
//! **Position:** inside the engine seam. The toolbar buttons, the keyboard shortcuts and the
//! headless harness all funnel through this one driver, so no path exists that a gate cannot take.
//! **Signals & state:** the document handle and the driver context are thread-local `!Send` `Rc`s
//! installed when the page loads; the dirty flag behind the unsaved-work guard lives with them.
//! **Invariants:** there is no second undo stack — the core owns a `yrs` `UndoManager` scoped to
//! the local origin, so only operator gestures are undoable and a seeded or hydrated document is
//! not. Every module here holds a live document handle, so each is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate.

/// The hosted document: its lifecycle, the deterministic seed the smoke tests drive, and the
/// read-only bridge that reports its shape.
#[cfg(target_arch = "wasm32")]
pub mod doc_host;
/// The undo/redo driver over the hosted document's own stack, the post-change rebind of every
/// render lane, and the guard that warns before a tab close discards unsaved work.
#[cfg(target_arch = "wasm32")]
pub mod history;
