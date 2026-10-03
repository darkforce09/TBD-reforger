//! Why a ticket model call failed.
//!
//! **Role:** the crate's error type, its `Result` alias and the crate-private [`ResultExt`] that
//! adds a context line to a failure.
//! **Position:** returned by the commit-subject miner ([`crate::commit_subjects`]); the parsers and
//! the store answer a one-line `String`, which the callers print as it stands.
//! **Signals & state:** none; plain data.
//! **Invariants:** a [`Error::Context`] displays its own line only and keeps the failure it wraps
//! as its source, so a caller that prints the chain (`{error:#}` through `anyhow`, or
//! [`crate::error_chain_text`]) prints `context: cause` exactly once.

use std::fmt::Display;

/// Why a ticket model call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted.
    #[error("{context}")]
    Context {
        /// The step that failed, as the command output names it.
        context: String,
        /// The failure underneath.
        #[source]
        cause: Box<Error>,
    },
    /// `git` could not be run, or died without an exit code.
    #[error(transparent)]
    Process(#[from] process_runner::Error),
    /// A commit date is not RFC 3339.
    #[error(transparent)]
    DateParse(#[from] time::error::Parse),
    /// A normalised stamp breaks the UTC rule.
    #[error(transparent)]
    Timestamp(#[from] time_source::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Adds the step that failed to an error, or turns a missing value into a refusal.
pub(crate) trait ResultExt<T> {
    /// Wraps the failure under `context`.
    fn context(self, context: impl Display) -> Result<T>;
    /// Wraps the failure under the context `make` builds, only when there is a failure.
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
        self.ok_or_else(|| Error::Message(context.to_string()))
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.ok_or_else(|| Error::Message(make().to_string()))
    }
}
