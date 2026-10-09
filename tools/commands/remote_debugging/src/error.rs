//! Why a debug or repro command failed.
//!
//! **Role:** the crate's [`Error`] and [`Result`], and the crate-private [`ResultExt`] that puts
//! the step that failed (`open <log>`, `parse JSON`) in front of a failure.
//! **Position:** returned by every fallible entry of the crate; `xtask` prints it through
//! `anyhow` as `xtask: {error:#}` and exits 1.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant's text is complete on its own: a [`Error::Context`] displays
//! `<step>: <cause>` and exposes no source, so `{error:#}` and `{error}` print the same line, the
//! text the commands have always printed. A probe's or a verdict's outcome is its exit code, never
//! an error: an error means the command could not run.

use std::fmt::Display;

/// Why a debug or repro command failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A failure the command describes in full (`no ports`, `missing id`).
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
    /// A file, folder or standard stream operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document did not parse or a row did not serialize.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A `SIZES_MB` entry is not an integer.
    #[error(transparent)]
    Integer(#[from] std::num::ParseIntError),
    /// No checkout root above the working directory.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

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
