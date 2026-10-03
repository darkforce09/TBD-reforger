//! The toasts a command leaves behind.
//!
//! **Role:** `Toast`, a message with its error flag and expiry.
//! **Position:** part of `crate::ticket_actions::models`; pushed by the desktop application's command
//! execution and painted by its feedback views.
//! **Signals & state:** none; plain data with its expiry instant.
//! **Invariants:** a toast expires on its own; it never blocks a control.

use super::*;
const TOAST_SECS: u64 = 6;

// ---- toasts ----

/// A transient notification in the window corner.
pub struct Toast {
    /// The text shown.
    pub text: String,
    /// True for a failure, painted in the error tone.
    pub error: bool,
    /// When the toast disappears.
    pub until: Instant,
}

impl Toast {
    /// A toast showing `text` for six seconds.
    pub fn new(text: String, error: bool) -> Self {
        Self {
            text,
            error,
            until: Instant::now() + std::time::Duration::from_secs(TOAST_SECS),
        }
    }
}
