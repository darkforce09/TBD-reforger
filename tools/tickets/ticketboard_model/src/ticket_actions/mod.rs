//! Ticket commands: requests, transitions, file-change guards, the queue, dialogs and toasts.
//!
//! **Role:** builds the `cargo xtask ticket <verb>` command lines, the dialogs that collect their
//! inputs, the compare-and-set guard over the ticket file, the single-flight queue and the
//! execution and notification state.
//! **Position:** over [`crate::ticket_registry`] and [`crate::core::process`]; the application runs
//! the commands, and `tools/tickets/ticketboard_desktop`'s `ticket_actions::ui` paints the dialogs
//! and feedback and emits [`events::TicketActionEvent`]s.
//! **Signals & state:** plain data; the running command's subprocess handle lives in
//! [`models::CommandExecutionState`].
//! **Invariants:** tickets change only through `cargo xtask ticket` subprocesses, one at a time;
//! a guard refuses a command whose ticket file changed since it was offered.

pub mod events;
pub mod models;
pub mod services;
