//! Why a database lane, backup, restore, drill or database check call failed.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private [`ResultExt`] that adds
//! a context line to a failure or turns a missing value into a refusal, and the crate-private
//! `refuse!` and `stop!` macros that return a refusal or an operator stop built like `format!`.
//! **Position:** returned by every public entry of the crate; the xtask binary prints an
//! [`Error::Stop`] bare and exits 1, and prints every other error with `xtask: {error:#}` and
//! exits 1. A verb's or a gate's verdict is its exit code, not an error.
//! **Signals & state:** none; plain data.
//! **Invariants:** a [`Error::Context`] displays `context: cause` with the cause's whole source
//! chain and exposes no source, so its text is the full chain whether a caller prints it directly
//! or through `anyhow` with `{error:#}`; an [`Error::Stop`] passes through every added context
//! unchanged, so the operator always reads the stop's own words.

use std::fmt::Display;

/// Why a database lane, backup, restore, drill or database check call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// An operator stop: the verb refuses to go on, and the report is printed bare (no `xtask:`
    /// prefix) with exit 1. The report is the whole text, including its last newline.
    #[error("{}", report.trim_end_matches('\n'))]
    Stop {
        /// The text printed to stderr, verbatim.
        report: String,
    },
    /// A failure with the step it interrupted; its text embeds the cause's text and chain.
    #[error("{context}: {}", chain_text(cause.as_ref()))]
    Context {
        /// The step that failed, as the command output names it.
        context: String,
        /// The failure underneath, part of this error's own text rather than its source.
        cause: Box<Error>,
    },
    /// A file could not be read or written, or a child could not be spawned.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// A regular expression does not compile.
    #[error(transparent)]
    Pattern(#[from] regex::Error),
    /// A program could not be run, or died without an exit code.
    #[error(transparent)]
    Process(#[from] process_runner::Error),
    /// A program or a scanned file gave no answer: absent, unreadable, signalled or timed out.
    #[error(transparent)]
    NotRun(#[from] verification_core::NotRun),
    /// The property-test settings of an integration run are invalid.
    #[error(transparent)]
    PropertyTestConfiguration(#[from] api_readiness_checks::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// The operator stop `FATAL: <message>`, printed on one line.
    pub fn fatal(message: impl Display) -> Self {
        Error::Stop {
            report: format!("FATAL: {message}\n"),
        }
    }

    /// The operator stop whose report is `report` exactly, printed with no line added.
    pub fn stop(report: impl Into<String>) -> Self {
        Error::Stop {
            report: report.into(),
        }
    }

    /// The text to print for an operator stop, or `None` for every other error.
    pub fn stop_report(&self) -> Option<&str> {
        match self {
            Error::Stop { report } => Some(report),
            _ => None,
        }
    }

    /// Whether this is an operator stop, which no caller in the crate turns into anything else.
    pub(crate) fn is_stop(&self) -> bool {
        matches!(self, Error::Stop { .. })
    }

    /// The outermost line of this error: a [`Error::Context`]'s step alone, any other error's
    /// whole text (what `anyhow` prints for `{error}` without `#`).
    pub(crate) fn outermost_text(&self) -> String {
        match self {
            Error::Context { context, .. } => context.clone(),
            other => other.to_string(),
        }
    }

    /// This error under `context` (an operator stop stays as it is).
    pub(crate) fn under(self, context: impl Display) -> Error {
        wrapped(context.to_string(), self)
    }
}

/// The text of `error` followed by every source below it, joined with `: `, as `anyhow` prints a
/// chain with `{error:#}`.
fn chain_text(error: &Error) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Returns a refusal ([`Error::Message`]) whose text is built like `format!`.
macro_rules! refuse {
    ($($argument:tt)*) => {
        return Err($crate::error::Error::msg(format!($($argument)*)))
    };
}
pub(crate) use refuse;

/// Returns the operator stop `FATAL: …` ([`Error::fatal`]) whose message is built like `format!`.
macro_rules! stop {
    ($($argument:tt)*) => {
        return Err($crate::error::Error::fatal(format!($($argument)*)))
    };
}
pub(crate) use stop;

/// Adds the step that failed to an error, or turns a missing value into a refusal.
pub(crate) trait ResultExt<T> {
    /// Wraps the failure under `context`.
    fn context(self, context: impl Display) -> Result<T>;
    /// Wraps the failure under the context `make` builds, only when there is a failure.
    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T>;
}

/// Wraps `cause` under `context`, except an operator stop, which stays as it is.
fn wrapped(context: String, cause: Error) -> Error {
    if cause.is_stop() {
        return cause;
    }
    Error::Context {
        context,
        cause: Box::new(cause),
    }
}

impl<T, E: Into<Error>> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, context: impl Display) -> Result<T> {
        self.map_err(|cause| wrapped(context.to_string(), cause.into()))
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.map_err(|cause| wrapped(make().to_string(), cause.into()))
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
