//! Why a platform command could not run.
//!
//! **Role:** the crate's [`Error`] and its [`Result`] alias.
//! **Position:** returned by [`crate::run`], [`crate::slice_execution::run_slice`] and
//! [`crate::slice_worktree::run_at`]; the xtask binary prints it with `xtask: {error:#}` and exits
//! with code 1. A gate's or a guard's verdict is its exit code, not an error: an error means the
//! command could not do its work at all.
//! **Signals & state:** none; plain data.
//! **Invariants:** a wrapped error of another crate keeps its own text and source chain; a
//! context names the operation that failed and keeps the failure as its source, so the printed
//! chain reads `<operation>: <cause>`.

use verification_core::NotRun;

/// Why a platform command could not run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A named file operation (`read …`, `write …`, `mkdir -p …`) failed.
    #[error("{context}")]
    File {
        /// The operation, as the refusal names it.
        context: String,
        /// The operating system's reason.
        #[source]
        source: std::io::Error,
    },
    /// The working directory could not be changed, or the report could not be written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A named JSON document could not be parsed.
    #[error("{context}")]
    Json {
        /// The document, as the refusal names it.
        context: String,
        /// The parser's reason.
        #[source]
        source: serde_json::Error,
    },
    /// A record could not be encoded as JSON.
    #[error(transparent)]
    JsonEncoding(#[from] serde_json::Error),
    /// A program could not be started, or died on a signal or a deadline.
    #[error("{context}")]
    ProgramNotRun {
        /// The run, as the refusal names it.
        context: String,
        /// Why the child did not run to an exit code.
        #[source]
        source: NotRun,
    },
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// The ticket registry could not be read.
    #[error(transparent)]
    TicketRegistry(#[from] ticket_registry::Error),
    /// A run receipt could not be written.
    #[error(transparent)]
    TicketMetrics(#[from] ticket_metrics::Error),
    /// A run reported no token usage, so no receipt was written.
    #[error("{context}")]
    NoTokenUsage {
        /// The refusal naming the slice.
        context: String,
        /// Why the agent's output carries no usage object.
        #[source]
        source: ticket_metrics::Error,
    },
    /// The committed wave lock could not be read.
    #[error(transparent)]
    TicketWaveLock(#[from] ticket_wave_lock::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub(crate) fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// A failed file operation named by `context`.
    pub(crate) fn file(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::File {
            context: context.into(),
            source,
        }
    }
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
