//! The argument of `console_command`: one line for the game server's console.
//!
//! **Role:** Validates the `line` argument before anything reaches RCON, by the rules the API
//! stores it under: 1 to [`CONSOLE_LINE_MAX_BYTES`] bytes, not blank, no control character and
//! no Unicode line or paragraph separator (so exactly one line), and no `@` as its first
//! character after any leading whitespace, since `@` starts the custom RCON commands, such as
//! `@logout`, which would end the agent's login.
//!
//! **Position:** `super::host_command` builds a [`ConsoleLine`] from a claimed `console_command`'s
//! arguments; `super::host_action_executor` transmits it once through
//! [`crate::rcon::RconClient::execute_once`].
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** the arguments hold exactly the key `line`; a [`ConsoleLine`] exists only for
//! text that passed every rule, and that text reaches the server verbatim in one RCON packet,
//! never through a shell.

use serde_json::Value;

use super::command_refusal::{CommandRefusal, arguments_with_only};
use crate::rcon::reforger_commands::RCON_CUSTOM_COMMAND_PREFIX;

pub(super) const CONSOLE_COMMAND: &str = "console_command";

/// Longest console line, in bytes.
pub const CONSOLE_LINE_MAX_BYTES: usize = 256;

const LINE_KEY: &str = "line";
const LINE_EXPECTATION: &str =
    "one line of 1 to 256 bytes, not blank, without control characters or a leading @";

/// One line for the game server's console that passed every rule of [`ConsoleLine::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLine(String);

impl ConsoleLine {
    /// `raw` as a console line: at most [`CONSOLE_LINE_MAX_BYTES`] bytes holding no control
    /// character, line separator or paragraph separator, whose first character after any
    /// leading whitespace exists and is not `@`.
    pub fn parse(raw: &str) -> Option<Self> {
        let first_visible = raw.trim_start().chars().next();
        let valid = raw.len() <= CONSOLE_LINE_MAX_BYTES
            && !raw.chars().any(is_control_or_line_separator)
            && first_visible.is_some_and(|first| first != RCON_CUSTOM_COMMAND_PREFIX);
        valid.then(|| Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `console_command` arguments: exactly the key `line`, holding a valid console line.
    pub(super) fn from_arguments(arguments: &Value) -> Result<Self, CommandRefusal> {
        arguments_with_only(CONSOLE_COMMAND, arguments, &[LINE_KEY])?
            .get(LINE_KEY)
            .and_then(Value::as_str)
            .and_then(Self::parse)
            .ok_or(CommandRefusal::InvalidArgument {
                action: CONSOLE_COMMAND,
                key: LINE_KEY,
                expected: LINE_EXPECTATION,
            })
    }
}

/// A control character (line feed, carriage return and the rest of Unicode's Cc category) or
/// Unicode's line separator (U+2028) or paragraph separator (U+2029).
fn is_control_or_line_separator(character: char) -> bool {
    character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
#[path = "tests/console_line.rs"]
mod tests;
