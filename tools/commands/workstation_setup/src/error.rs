//! Why a workstation setup command failed.
//!
//! **Role:** the crate's [`Error`] and [`Result`], and the crate-private [`ResultExt`] that puts
//! the step that failed (`mkdir -p <path>`, `parse <file>`) in front of a failure.
//! **Position:** returned by every command of the crate; `xtask` prints it through `anyhow`.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant's text is complete on its own: a [`Error::Context`] displays
//! `<step>: <cause>` and exposes no source, so `{error:#}` and `{error}` print the same line, the
//! text the commands have always printed. A refused input is an exit code, never an error.

use std::fmt::Display;

/// Why a workstation setup command failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A failure the command describes in full.
    #[error("{0}")]
    Refused(String),
    /// A failure with the step that hit it in front of it.
    #[error("{context}: {cause}")]
    Context {
        /// The step that failed.
        context: String,
        /// The failure itself; its text follows the step.
        cause: Box<Error>,
    },
    /// A file, folder or symlink operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The backend config is not JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A found pak is not under the addons folder it was found in.
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    /// A tool's output is not UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    /// A process environment variable is unset or not Unicode.
    #[error(transparent)]
    Variable(#[from] std::env::VarError),
    /// A shell tool could not be run to an exit code.
    #[error(transparent)]
    NotRun(#[from] verification_core::NotRun),
    /// No checkout root above the working directory.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_layout::Error),
}

/// The result of a workstation setup command.
pub type Result<T> = std::result::Result<T, Error>;

/// Puts the step that failed in front of a failure, or turns a missing value into a failure.
pub(crate) trait ResultExt<T> {
    /// The failure behind `context`.
    fn context(self, context: impl Display) -> Result<T>;
    /// The failure behind the context `make` builds, built only on failure.
    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T>;
}

impl<T, E: Into<Error>> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, context: impl Display) -> Result<T> {
        self.map_err(|cause| Error::Context {
            context: context.to_string(),
            cause: Box::new(cause.into()),
        })
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.map_err(|cause| Error::Context {
            context: make().to_string(),
            cause: Box::new(cause.into()),
        })
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn context(self, context: impl Display) -> Result<T> {
        self.ok_or_else(|| Error::Refused(context.to_string()))
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.ok_or_else(|| Error::Refused(make().to_string()))
    }
}
