//! A staging unit's journal since a moment: the log lines systemd kept for it.
//!
//! **Role:** builds the `journalctl --user` read of one unit from a Unix time on, and splits the
//! `short-unix` lines into their time and message.
//!
//! **Position:** used by effects that wait for a unit's own log line (an agent's claim, the API's
//! reconciliation outcome) and by the recovery checks.
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** the read only reads; `--since` takes whole seconds, so a line of the second the
//! step started is kept rather than lost.

use super::remote_command::{RemoteCommand, shell_words};

/// One journal line: its Unix time in milliseconds and the rest of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JournalLine {
    pub unix_ms: u64,
    pub text: String,
}

/// The journal of `unit` from `since_unix_seconds` on, oldest first.
pub(crate) fn since(unit: &str, since_unix_seconds: u64) -> RemoteCommand {
    let words = [
        "journalctl".to_string(),
        "--user".into(),
        format!("--unit={unit}"),
        format!("--since=@{since_unix_seconds}"),
        "--no-pager".into(),
        "--quiet".into(),
        "--output=short-unix".into(),
    ];
    RemoteCommand::read("unit journal", shell_words(&words))
}

/// The lines of `output` whose first word is a `short-unix` time (`<seconds>.<micros>`).
pub(crate) fn parse(output: &str) -> Vec<JournalLine> {
    output
        .lines()
        .filter_map(|line| {
            let (stamp, text) = line.split_once(' ')?;
            let (seconds, fraction) = stamp.split_once('.').unwrap_or((stamp, "0"));
            let seconds: u64 = seconds.parse().ok()?;
            let milliseconds: u64 = format!("{fraction:0<3}")[..3].parse().ok()?;
            Some(JournalLine {
                unix_ms: seconds * 1000 + milliseconds,
                text: text.to_string(),
            })
        })
        .collect()
}
