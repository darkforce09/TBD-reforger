//! Why a contract codegen, schema gate, font-table or flattening call failed.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private [`ResultExt`] that adds
//! a context line to a failure or turns a missing value into a refusal, and the crate-private
//! `refuse!` macro that returns a refusal built like `format!`.
//! **Position:** returned by every public entry of the crate; the xtask binary's `schema`, `gen`
//! and `ci` groups print it with `xtask: {error:#}` and exit 1. A gate's verdict is its exit code,
//! not an error: an error means the gate could not run.
//! **Signals & state:** none; plain data.
//! **Invariants:** a [`Error::Context`] displays `context: cause` and exposes no source, so its
//! text is the whole chain whether a caller prints it directly or through `anyhow` with
//! `{error:#}`, and the cause is printed exactly once; a refusal's text is the operator's
//! diagnosis, fixed per site.

use std::fmt::Display;

/// Why a contract codegen, schema gate, font-table or flattening call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted; its text embeds the cause's text.
    #[error("{context}: {cause}")]
    Context {
        /// The step that failed, as the command output names it.
        context: String,
        /// The failure underneath, part of this error's own text rather than its source.
        cause: Box<Error>,
    },
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document could not be parsed or written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryLayout(#[from] repository_layout::Error),
    /// A regular expression built from a scanned name does not compile.
    #[error(transparent)]
    Pattern(#[from] regex::Error),
    /// A walked path does not lie under the folder it was walked from.
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    /// A folder walk failed.
    #[error(transparent)]
    Walk(#[from] walkdir::Error),
    /// `rustfmt` could not be run, or died without an exit code.
    #[error(transparent)]
    Process(#[from] process_runner::Error),
    /// The ticket registry's empty-write refusal stopped a flattening write.
    #[error(transparent)]
    TicketRegistry(#[from] ticket_registry::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Returns a refusal ([`Error::Message`]) whose text is built like `format!`.
macro_rules! refuse {
    ($($argument:tt)*) => {
        return Err($crate::error::Error::msg(format!($($argument)*)))
    };
}
pub(crate) use refuse;

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
