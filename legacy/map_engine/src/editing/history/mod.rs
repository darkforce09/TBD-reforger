//! Role: Module boundary for the undo drive.
//! Position: `editing` in the map engine.
//! Signals & state: the hosted document's own undo stack, and the host services around it.
//! Invariants: there is no second stack. The document owns an undo manager scoped to the LOCAL
//! origin, so only operator gestures are undoable — a boot seed, a restore and a hydrate are not.
//! This module is the only path: toolbar buttons, keyboard shortcuts and test bridges all funnel
//! through [`undo`] / [`redo`], so no caller can take a route the gates have not proved.

/// The host services the undo drive cannot supply itself.
pub mod host;

/// Undo, redo, and the post-change tail every committed edit runs.
pub mod drive;

pub use drive::{after_local_edit, redo, undo};
pub use host::{HistoryHost, install_host};
