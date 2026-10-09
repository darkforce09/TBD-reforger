//! Duplicate detection for server messages.
//!
//! The server sends a server message again until the client acknowledges it, so a lost
//! acknowledgement brings the same message back. Every copy is acknowledged; only the first
//! copy is delivered to the log.

use std::collections::VecDeque;

/// Sequence numbers remembered. The server numbers its messages with one byte that wraps, and
/// repeats an unacknowledged message within seconds, so a window far shorter than 256
/// messages recognises every repetition without mistaking a reused number for one.
const WINDOW: usize = 64;

#[derive(Debug, Default)]
pub(super) struct ServerMessageWindow {
    recent: VecDeque<u8>,
}

impl ServerMessageWindow {
    /// True the first time `sequence` arrives within the window.
    pub(super) fn first_delivery(&mut self, sequence: u8) -> bool {
        if self.recent.contains(&sequence) {
            return false;
        }
        if self.recent.len() == WINDOW {
            self.recent.pop_front();
        }
        self.recent.push_back(sequence);
        true
    }

    /// Forgets every number: a new login starts the server's message numbering again.
    pub(super) fn clear(&mut self) {
        self.recent.clear();
    }
}

#[cfg(test)]
#[path = "tests/server_message_window.rs"]
mod tests;
