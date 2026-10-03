//! The running ticket command and its outcome.
//!
//! **Role:** `CommandExecutionState` (the queue, the running handle, the output, the last outcome,
//! the drawer) and `CommandOutcome`.
//! **Position:** part of `crate::ticket_actions::models`; owned by the desktop application's command
//! execution and painted by its chip and drawer.
//! **Signals & state:** the command session, owned by the application.
//! **Invariants:** at most one command runs; a failure drops the queued tail and nothing retries.

use super::*;
// ---- verb runner state (owned by the app, drained in app::poll_verb) ----

/// Everything the running-verb surface needs: the single-flight queue, the
/// in-flight subprocess, the FULL verbatim merged log, and the last outcome.
pub struct CommandExecutionState {
    /// The commands waiting to run, one at a time.
    pub queue: TicketCommandQueue,
    /// The running command's subprocess.
    pub handle: Option<ProcessHandle>,
    /// FULL merged stdout+stderr of the current / most recent verb run —
    /// unbounded on purpose (verb output is small; refusals must never truncate)
    /// and painted virtualized.
    pub log: Vec<String>,
    /// How the last command ended.
    pub last: Option<CommandOutcome>,
    /// True when the output drawer is open.
    pub drawer_open: bool,
    /// "N pending request(s) dropped" note after a failure — nothing auto-retries.
    pub dropped_note: Option<String>,
}

impl Default for CommandExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutionState {
    /// An idle state: empty queue, nothing running, drawer closed.
    pub fn new() -> Self {
        Self {
            queue: TicketCommandQueue::default(),
            handle: None,
            log: Vec::new(),
            last: None,
            drawer_open: false,
            dropped_note: None,
        }
    }
}

/// A finished verb run — the drawer headline.
pub struct CommandOutcome {
    /// The literal command line that ran.
    pub display: String,
    /// Exit code; `None` = killed by a signal (the mid-verb SIGKILL case).
    pub code: Option<i32>,
    /// Completion wall time, `"HH:MM:SS UTC"`.
    pub at: String,
    /// The spawn itself failed — the verb never ran.
    pub spawn_error: Option<String>,
    /// The log carries the wave-stale signature → show [`crate::ticket_actions::services::commands::RECOVERY_HINT`].
    pub hint: bool,
}
