//! What the browser views ask the application to do.
//!
//! **Role:** `BrowserEvent`: select, compare, toggle columns, nodes and the quarantine block, open a
//! path or document, copy text, forward ticket actions.
//! **Position:** emitted by the desktop application's board, tree and detail column; converted into
//! an `Action` by `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** the browser reads the registry and never changes it; ticket changes travel as
//! `TicketAction`.

use std::path::PathBuf;
/// What the browser views ask the application to do.
pub enum BrowserEvent {
    /// Select the ticket at this corpus index.
    Select(usize),
    /// Select the ticket with this id.
    SelectId(String),
    /// Pick the ticket at this corpus index as the comparison.
    Compare(usize),
    /// Clear the comparison.
    ClearCompare,
    /// Expand or collapse the status column at this index.
    ToggleColumn(usize),
    /// Open this path with the operating system's handler.
    OpenPath(PathBuf),
    /// Open this repository-relative document in the viewer.
    OpenDoc(String),
    /// Close the detail column.
    CloseDetail,
    /// Expand or collapse the tree node at this corpus index.
    ToggleNode(usize),
    /// Copy this text to the clipboard.
    CopyText(String),
    /// Expand or collapse the quarantined parked lines.
    ToggleQuarantineExpand,
    /// Open the anchor picker for the ticket at this corpus index.
    OpenAnchorDialog(usize),
    /// A ticket-command event from a menu or the action strip.
    TicketAction(crate::ticket_actions::events::TicketActionEvent),
}
