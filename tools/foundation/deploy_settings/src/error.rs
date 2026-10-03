//! Why the deploy settings could not be loaded, and why a setting cannot be used.
//!
//! **Role:** the crate's [`Error`] (the file could not be loaded) and its `Result` alias, and
//! [`SettingError`] (a loaded setting a command cannot use) with the [`SettingOrigin`] it points at.
//! **Position:** returned by [`crate::DeployEnvironment`]'s loaders and accessors and by
//! [`crate::DeployHostFolder::resolve`]; a command on `anyhow` converts both with `?` and prints
//! their text as it stands.
//! **Signals & state:** none; plain data.
//! **Invariants:** a refusal names the file and, for a value the file assigns, the line; no message
//! ever echoes a value, which may be a secret; the error text is the operator's instruction, so
//! each variant's wording is fixed.

use std::io;
use std::path::PathBuf;

/// Why the deploy settings file could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No file at the path, for a command that needs one.
    #[error(
        "Missing {} — copy from {}",
        path.display(),
        repository_layout::DEPLOY_ENV_EXAMPLE
    )]
    Missing {
        /// The settings file that does not exist.
        path: PathBuf,
    },
    /// The file exists and reading it failed.
    #[error("could not read {}: {error}", path.display())]
    Unreadable {
        /// The settings file that could not be read.
        path: PathBuf,
        /// The read failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// A line breaks the `KEY=VALUE` grammar.
    #[error("{}:{line}: {message}", path.display())]
    Syntax {
        /// The settings file holding the line.
        path: PathBuf,
        /// The 1-based number of the line.
        line: usize,
        /// What is wrong with the line; it never echoes the value.
        message: String,
    },
}

/// The result of loading the deploy settings file.
pub type Result<T> = std::result::Result<T, Error>;

/// Where a setting's value came from, so an error points at the edit to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingOrigin {
    /// Assigned in the deploy settings file, on this line.
    File {
        /// The settings file.
        path: PathBuf,
        /// The 1-based line of the assignment.
        line: usize,
    },
    /// Taken from the process environment, because the file never assigns the key.
    ProcessEnvironment,
}

/// A setting a command cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingError {
    /// The key is unset, or assigned empty, and the command needs it.
    #[error("{key} is not set: add it to {}", path.display())]
    Missing {
        /// The setting's key.
        key: String,
        /// The settings file the key belongs in.
        path: PathBuf,
    },
    /// The key holds a value the command refuses.
    #[error("{}", invalid_setting_text(key, origin, problem))]
    Invalid {
        /// The setting's key.
        key: String,
        /// Where the refused value came from.
        origin: SettingOrigin,
        /// Why the command refuses it.
        problem: String,
    },
    /// The key is unset and the value it defaults to cannot be derived from here.
    #[error(
        "{key} is unset and {reason}: {remedy} or set {key} in {}",
        path.display()
    )]
    NotDerivable {
        /// The setting's key.
        key: String,
        /// The settings file the key belongs in.
        path: PathBuf,
        /// Why the default cannot be derived.
        reason: String,
        /// What the operator can do instead of setting the key.
        remedy: String,
    },
}

/// The text of [`SettingError::Invalid`]: `<path>:<line>: <key>: <problem>` for a value the file
/// assigns, `<key> (process environment): <problem>` for one the environment supplies.
fn invalid_setting_text(key: &str, origin: &SettingOrigin, problem: &str) -> String {
    match origin {
        SettingOrigin::File { path, line } => {
            format!("{}:{line}: {key}: {problem}", path.display())
        }
        SettingOrigin::ProcessEnvironment => format!("{key} (process environment): {problem}"),
    }
}
