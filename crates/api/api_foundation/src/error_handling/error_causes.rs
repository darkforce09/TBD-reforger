//! The message of an error followed by every cause beneath it, on one line.
//!
//! **Role:** renders an error for a log line or a progress message that has to carry the whole
//! cause chain, not only the outermost message.
//! **Position:** `api_foundation::error_handling`; the Discord OAuth handlers and the equipment export
//! watcher render the typed errors of `api_discord` and `api_equipment_datasets` with it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the outermost message comes first, then each `source()` in order, joined by
//! `": "`; an error with no source renders as its own message alone.

use std::error::Error;

/// `error`'s message followed by the message of each of its `source()` causes, joined by `": "`.
pub fn message_with_causes(error: &dyn Error) -> String {
    let mut rendered = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        rendered.push_str(": ");
        rendered.push_str(&next.to_string());
        cause = next.source();
    }
    rendered
}
