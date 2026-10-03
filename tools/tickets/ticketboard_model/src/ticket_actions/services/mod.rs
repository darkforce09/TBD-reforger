//! The ticket action rules.
//!
//! **Role:** declares `commands` (builders, guards, queue, transitions) and `dialog_builders`.
//! **Position:** used by `crate::ticket_actions::models` and by the desktop application.
//! **Signals & state:** none here; see each module.
//! **Invariants:** nothing here names egui or writes a ticket file.

pub mod commands;
pub mod dialog_builders;
