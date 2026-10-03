//! The dialogs, the running command, the mutation contexts and the toasts.
//!
//! **Role:** re-exports `CommandExecutionState`, `CommandOutcome`, `Dialog`, `MutationContext`,
//! `TicketActionContext` and `Toast`.
//! **Position:** built by `crate::ticket_actions::services::dialog_builders`; owned by the desktop
//! application, which lends them to its menus, dialogs and drawer.
//! **Signals & state:** `CommandExecutionState` holds the running command's `ProcessHandle`, log and
//! queue.
//! **Invariants:** a dialog sees only the corpus and the id index, never application state; every
//! control that could dispatch is disabled while a command runs.

use crate::{
    core::process::ProcessHandle,
    ticket_actions::services::commands::{FileChangeGuard, TicketCommand, TicketCommandQueue},
    ticket_registry::models::corpus::Corpus,
};
use std::{collections::HashMap, path::Path, time::Instant};
use ticket_model::TicketId;
mod command_execution;
pub use command_execution::*;
mod dialog;
pub use dialog::*;
mod mutation_context;
pub use mutation_context::*;
mod notification;
pub use notification::*;
