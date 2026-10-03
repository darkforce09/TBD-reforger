//! The borrowed view the status banner paints.
//!
//! **Role:** `StatusView`: the check model, its output log, the watch errors and the `git status`
//! chip for one frame.
//! **Position:** built by the desktop application and painted by its status banner.
//! **Signals & state:** none; the view only borrows.
//! **Invariants:** painting cannot change the check, the log or the chip.

use super::{check_status::CheckModel, git_status::GitChip};
use crate::core::process::BoundedLog;
/// What the status banner reads each frame, borrowed from the application.
pub struct StatusView<'a> {
    /// The strict-check model.
    pub check: &'a CheckModel,
    /// True while a strict check runs.
    pub check_running: bool,
    /// The check's bounded output.
    pub check_log: &'a BoundedLog,
    /// True when the verbatim output pane is open.
    pub show_output: bool,
    /// Why the registry watch failed to arm, when it did.
    pub watch_error: Option<&'a str>,
    /// The optional watch targets that could not be armed.
    pub degraded_watches: &'a [String],
    /// The `git status` chip.
    pub git_chip: &'a GitChip,
    /// True when the changed-file list is expanded.
    pub git_expanded: bool,
}
