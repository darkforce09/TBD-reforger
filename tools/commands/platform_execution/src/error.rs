//! Why a platform command could not run.
//!
//! **Role:** the crate's [`Error`] and its [`Result`] alias, and `error_chain_text`, the one-line
//! rendering of an error and its sources that the wave driver prints.
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
    /// The central ticket manager could not answer, or refused.
    #[error(transparent)]
    TicketManager(#[from] ticket_manager_client::Error),
    /// A run reported no token usage, so no receipt was recorded.
    #[error("{context}")]
    NoTokenUsage {
        /// The refusal naming the slice.
        context: String,
        /// Why the agent's output carries no usable usage object.
        #[source]
        source: crate::slice_execution::token_usage::Error,
    },
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

/// An error and every source under it, joined as `outer: inner: …`, for the refusal lines the
/// wave driver prints.
pub(crate) fn error_chain_text(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
