use super::*;
// ---- verb runner state (owned by the app, drained in app::poll_verb) ----

/// Everything the running-verb surface needs: the single-flight queue, the
/// in-flight subprocess, the FULL verbatim merged log, and the last outcome.
pub struct CommandExecutionState {
    pub queue: TicketCommandQueue,
    pub handle: Option<ProcessHandle>,
    /// FULL merged stdout+stderr of the current / most recent verb run —
    /// unbounded on purpose (verb output is small; refusals must never truncate)
    /// and painted virtualized.
    pub log: Vec<String>,
    pub last: Option<CommandOutcome>,
    pub drawer_open: bool,
    /// "N pending request(s) dropped" note after a failure — nothing auto-retries.
    pub dropped_note: Option<String>,
}

impl CommandExecutionState {
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
