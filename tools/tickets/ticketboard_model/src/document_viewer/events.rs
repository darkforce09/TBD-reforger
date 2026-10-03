//! What the document column asks the application to do.
//!
//! **Role:** `DocumentEvent`: close the viewer, or open a path with the operating system's handler.
//! **Position:** emitted by the desktop application's document column; converted into an `Action`
//! by `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** closing the viewer never changes the ticket selection.

use std::path::PathBuf;
/// What the document column asks the application to do.
pub enum DocumentEvent {
    /// Collapse the document column, keeping the selection.
    CloseViewer,
    /// Open this path with the operating system's handler.
    OpenPath(PathBuf),
}
