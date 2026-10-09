//! Why a browser gate, the static server or the capture harness could not run.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private `ResultExt` that adds
//! the step that failed to an error or turns a missing value into a refusal, and the
//! crate-private macros `refusal!` (a refusal built like `format!`), `bail!` (return one) and
//! `ensure!` (return one when a condition does not hold).
//! **Position:** returned by every fallible entry of the crate; the `gate` and `capture` command
//! lines ([`crate::command_lines`]) print it with [`Error::with_causes`] after `gate: driver
//! error: ` or `capture: ` and exit 3 or 1. A gate's verdict is its exit code, not an error: an error means
//! the gate could not run.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays its step alone and exposes the failure
//! underneath as its source, so `{error}` prints the outermost step and a chain walk
//! ([`Error::with_causes`]) prints `step: cause: …`, each cause exactly once; a wrapped library
//! error displays and chains exactly as that library's error does.

use std::fmt::Display;

/// Why a browser gate, the static server or the capture harness could not run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted; the failure underneath is its source.
    #[error("{context}")]
    Context {
        /// The step that failed, as the gate output names it.
        context: String,
        /// The failure underneath.
        #[source]
        cause: Box<Error>,
    },
    /// A file or a folder could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document, a page answer or a verdict could not be parsed or written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// An HTTP request to the app, the API proxy or the browser failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// The browser could not be launched or driven.
    #[error(transparent)]
    Protocol(#[from] chrome_devtools_protocol::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// A pattern scanning page text or route sources does not compile.
    #[error(transparent)]
    Pattern(#[from] regex::Error),
    /// A captured screenshot could not be decoded, cropped or encoded.
    #[error(transparent)]
    Image(#[from] image::ImageError),
    /// A URL the data viewer gate builds does not parse.
    #[error(transparent)]
    Url(#[from] url::ParseError),
    /// The system clock reads before the Unix epoch.
    #[error(transparent)]
    Clock(#[from] std::time::SystemTimeError),
    /// A captured payload's base64 does not decode.
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    /// A ballistics catalog document does not decode.
    #[error(transparent)]
    Catalog(#[from] ballistics_model::catalog::CatalogDecodeError),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// This error followed by each of its causes after `: `, outermost first: the text the
    /// command lines print and the verdict lines that record a whole failure.
    #[must_use]
    pub fn with_causes(&self) -> String {
        let mut text = self.to_string();
        let mut cause = std::error::Error::source(self);
        while let Some(next) = cause {
            text.push_str(": ");
            text.push_str(&next.to_string());
            cause = next.source();
        }
        text
    }
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A refusal ([`Error::Message`]) whose text is built like `format!`.
macro_rules! refusal {
    ($($argument:tt)*) => {
        $crate::error::Error::msg(format!($($argument)*))
    };
}
pub(crate) use refusal;

/// Returns a refusal whose text is built like `format!`.
macro_rules! bail {
    ($($argument:tt)*) => {
        return Err($crate::error::refusal!($($argument)*))
    };
}
pub(crate) use bail;

/// Returns a refusal whose text is built like `format!` when `condition` does not hold.
macro_rules! ensure {
    ($condition:expr, $($argument:tt)*) => {
        let holds: bool = $condition;
        if !holds {
            $crate::error::bail!($($argument)*);
        }
    };
}
pub(crate) use ensure;

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
