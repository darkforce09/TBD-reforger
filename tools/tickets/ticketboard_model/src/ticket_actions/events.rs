//! What the ticket-command views ask the application to do.
//!
//! **Role:** `TicketActionEvent`: open a mutation dialog, dispatch a command, toggle the drawer.
//! **Position:** emitted by the desktop application's menus, dialogs and drawer, and forwarded by
//! the browser inside `BrowserEvent::TicketAction`; converted into an `Action` by
//! `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** a dispatch carries the command with the file-change guard taken when the action
//! was offered.

use crate::ticket_actions::{models::Dialog, services::commands::TicketCommand};
/// What the ticket-command views ask the application to do.
pub enum TicketActionEvent {
    /// Open this mutation dialog.
    OpenDialog(Box<Dialog>),
    /// Run this command: guard check, then the queue.
    Dispatch(TicketCommand),
    /// Show or hide the command output drawer.
    ToggleVerbDrawer,
}
