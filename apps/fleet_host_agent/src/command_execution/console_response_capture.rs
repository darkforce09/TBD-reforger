//! The outcome of `console_command`: the server's reply to the line, bounded for the ledger.
//!
//! **Role:** Keeps a reply of up to [`CONSOLE_RESPONSE_MAX_BYTES`] bytes verbatim and cuts a
//! longer one at the last character boundary at or before that bound, recording whether it was
//! cut.
//!
//! **Position:** `super::host_action_executor` captures the reply text that
//! [`crate::rcon::RconClient::execute_once`] returns and reports
//! [`ConsoleResponseCapture::into_outcome`] as the command's outcome, which the API refuses when
//! its `response` exceeds the bound.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** `response` is a prefix of the reply of at most [`CONSOLE_RESPONSE_MAX_BYTES`]
//! bytes of valid UTF-8; `response_truncated` is true exactly when the reply was longer than the
//! bound.

use serde_json::{Map, Value};

/// Longest `response` the ledger records for a console command, in bytes.
pub const CONSOLE_RESPONSE_MAX_BYTES: usize = 4096;

/// The server's reply to a console line as the ledger records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleResponseCapture {
    /// The whole reply, or its longest prefix of at most [`CONSOLE_RESPONSE_MAX_BYTES`] bytes that
    /// ends on a character boundary.
    pub response: String,
    /// True when the reply was longer than [`CONSOLE_RESPONSE_MAX_BYTES`] bytes and `response`
    /// holds only its start.
    pub response_truncated: bool,
}

impl ConsoleResponseCapture {
    /// Captures `reply` within the bound.
    pub fn of(reply: &str) -> Self {
        let end = reply.floor_char_boundary(CONSOLE_RESPONSE_MAX_BYTES);
        Self {
            response: reply[..end].to_owned(),
            response_truncated: end < reply.len(),
        }
    }

    /// The `console_command` outcome: `{"response": <text>, "response_truncated": <bool>}`.
    pub fn into_outcome(self) -> Map<String, Value> {
        let mut outcome = Map::new();
        outcome.insert("response".to_owned(), Value::from(self.response));
        outcome.insert(
            "response_truncated".to_owned(),
            Value::from(self.response_truncated),
        );
        outcome
    }
}

#[cfg(test)]
#[path = "tests/console_response_capture.rs"]
mod tests;
