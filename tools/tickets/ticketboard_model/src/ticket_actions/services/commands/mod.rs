//! The `cargo xtask ticket` command builders, guards, queue and transitions.
//!
//! **Role:** re-exports `TicketCommand` and the verb builders, `FileChangeGuard` and `cas_ok`,
//! `TicketCommandQueue`, the offered transitions, the recovery hint and the success tail.
//! **Position:** used by the rest of `crate::ticket_actions` and by the desktop application's
//! command execution.
//! **Signals & state:** `TicketCommandQueue` is the single-flight queue; the rest is pure.
//! **Invariants:** every argument list is `TICKET_PREFIX` plus one verb tail; nothing here writes a
//! ticket file or runs `cargo xtask wave repack`.

use crate::ticket_registry::models::projection as board;
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};
use ticket_model::{StatusName, TicketId};

mod requests;
pub use requests::*;
mod file_change_guard;
pub use file_change_guard::*;
mod queue;
pub use queue::*;
mod transitions;
pub use transitions::*;

#[cfg(test)]
#[path = "tests/commands.rs"]
mod tests;
