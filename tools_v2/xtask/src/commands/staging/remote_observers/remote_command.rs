//! One command for the staging host: what it is for, its shell command line and its stdin.
//!
//! **Role:** the value every observer and remote action builds and [`HostCommandRunner`] runs,
//! and the quoting that keeps every value one shell word.
//!
//! **Position:** built by `remote_observers/*` (reads) and `remote_actions/*` (changes); run by
//! `host_shell.rs` over ssh in a live run and by recorded fakes in tests.
//!
//! **Signals & state:** none; values and pure functions.
//!
//! **Invariants:** every value placed in a command line passes through [`shell_quote`], so no
//! value can end the word it sits in; secrets never enter a command line, only files the host
//! reads for itself; a [`CommandPurpose::Read`] command changes nothing on the host.

use anyhow::Result;

/// Whether a command only reads the staging host or changes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandPurpose {
    /// Reads only: an observation, a preflight check, a status line.
    Read,
    /// Changes the host: a confirmed action or a procedure's host action.
    Change,
}

/// A command for the staging host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteCommand {
    /// The observer or action label the journal records, e.g. `database` or `unit state`.
    pub observer: &'static str,
    /// Whether the command only reads.
    pub purpose: CommandPurpose,
    /// The complete shell command line the host's shell runs.
    pub command_line: String,
    /// What the command reads on stdin, when anything.
    pub stdin: Option<String>,
}

impl RemoteCommand {
    /// A read-only command without stdin.
    pub(crate) fn read(observer: &'static str, command_line: String) -> Self {
        Self {
            observer,
            purpose: CommandPurpose::Read,
            command_line,
            stdin: None,
        }
    }

    /// A host-changing command without stdin.
    pub(crate) fn change(observer: &'static str, command_line: String) -> Self {
        Self {
            observer,
            purpose: CommandPurpose::Change,
            command_line,
            stdin: None,
        }
    }

    /// A read-only `bash -s` script sent on stdin.
    pub(crate) fn read_script(observer: &'static str, script: String) -> Self {
        Self {
            stdin: Some(script),
            ..Self::read(observer, "bash -s".to_string())
        }
    }

    /// A host-changing `bash -s` script sent on stdin.
    pub(crate) fn change_script(observer: &'static str, script: String) -> Self {
        Self {
            stdin: Some(script),
            ..Self::change(observer, "bash -s".to_string())
        }
    }
}

/// What one command returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
    /// The remote exit code; 255 is ssh's own failure.
    pub exit_code: i32,
    /// Standard output, the observation's raw artifact.
    pub stdout: String,
    /// Standard error, shown to the operator after a failed action and never journaled.
    pub stderr: String,
}

/// Runs commands on the staging host: over ssh in a live run, from recordings in tests.
pub(crate) trait HostCommandRunner {
    /// Runs `command` and returns its output; errors only when it could not run at all.
    fn run(&mut self, command: &RemoteCommand) -> Result<CommandOutput>;
}

/// `value` as one POSIX shell word: single-quoted, each `'` written as `'\''`.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// `words`, each quoted, joined by spaces.
pub(crate) fn shell_words<S: AsRef<str>>(words: &[S]) -> String {
    words
        .iter()
        .map(|word| shell_quote(word.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}
