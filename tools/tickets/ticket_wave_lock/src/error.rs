//! Why a wave lock call failed.
//!
//! **Role:** the crate's error type, its `Result` alias and the crate-private [`ResultExt`] that
//! adds a context line to a failure.
//! **Position:** returned by the compiler, reader, writer and collision report;
//! `ticket_registry` and xtask convert it with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a [`Error::Context`] displays its own line only and keeps the failure it wraps
//! as its source, so a caller that prints the chain (`{error:#}` through `anyhow`, or
//! `ticket_model::error_chain_text`) prints `context: cause` exactly once.

use std::fmt::Display;

/// Why a wave lock call failed.
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
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// `git` could not be run, or died without an exit code.
    #[error(transparent)]
    Process(#[from] process_runner::Error),
    /// A ticket model call failed.
    #[error(transparent)]
    Model(#[from] ticket_model::Error),
    /// A TOML document could not be parsed.
    #[error(transparent)]
    TomlParse(#[from] toml::de::Error),
    /// A TOML document could not be written.
    #[error(transparent)]
    TomlRender(#[from] toml::ser::Error),
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
