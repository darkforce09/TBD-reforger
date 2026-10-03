//! The typed arguments and outcome of a console command.
//!
//! **Role:** the stored arguments of [`crate::fleet_action::FleetAction::ConsoleCommand`] and the
//! outcome its host agent reports, with their size limits.
//! **Position:** the API validates an operator's line into [`ConsoleCommandArguments`] and a
//! reported outcome into [`ConsoleCommandOutcome`]; both travel inside the free-form
//! `arguments` and `outcome` objects of [`crate::operator_messages`] and
//! [`crate::executor_messages`].
//! **Signals & state:** none; plain data.
//! **Invariants:** neither shape admits an unknown key; a stored line holds at most
//! [`ConsoleCommandArguments::LINE_MAX_BYTES`] bytes and a recorded response at most
//! [`ConsoleCommandOutcome::RESPONSE_MAX_BYTES`].
//!
//! @contract fleet-command.schema.json#/definitions/ConsoleCommandArguments
//! @contract fleet-command.schema.json#/definitions/ConsoleCommandOutcome

use serde::{Deserialize, Serialize};

/// The stored arguments of a console command: one validated line, trimmed of surrounding
/// whitespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsoleCommandArguments {
    /// The line the host agent sends to the server's RCON console.
    pub line: String,
}

impl ConsoleCommandArguments {
    /// Longest line, in UTF-8 bytes.
    pub const LINE_MAX_BYTES: usize = 256;
}

/// What the host agent reports for a succeeded console command: the server's reply, cut on a
/// character boundary at [`Self::RESPONSE_MAX_BYTES`], and whether it was cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsoleCommandOutcome {
    /// The server's reply.
    pub response: String,
    /// Whether the reply was cut at [`Self::RESPONSE_MAX_BYTES`].
    pub response_truncated: bool,
}

impl ConsoleCommandOutcome {
    /// Longest recorded response, in UTF-8 bytes.
    pub const RESPONSE_MAX_BYTES: usize = 4096;
}

#[cfg(test)]
#[path = "tests/console_command.rs"]
mod tests;
