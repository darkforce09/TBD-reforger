//! What the status banner asks the application to do.
//!
//! **Role:** `StatusEvent`: rerun or cancel the strict check, toggle its output, toggle the
//! `git status` file list.
//! **Position:** emitted by the desktop application's status banner; converted into an `Action` by
//! `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** the banner never reimplements the check; it only asks for it to run again.

/// What the status banner asks the application to do.
pub enum StatusEvent {
    /// Run the strict check again.
    Recheck,
    /// Kill the running strict check.
    CancelCheck,
    /// Show or hide the check's verbatim output.
    ToggleOutput,
    /// Expand or collapse the `git status` file list.
    ToggleGitList,
}
