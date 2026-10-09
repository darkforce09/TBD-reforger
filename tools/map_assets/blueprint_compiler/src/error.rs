//! Why a blueprint compiler command could not finish.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private `ResultExt` that adds
//! the step that failed to an error or turns a missing value into a refusal, and the
//! crate-private macros `refusal!` (a refusal built like `format!`) and `bail!` (return one).
//! **Position:** returned by every fallible function of the crate; the `cargo xtask map` adapters
//! convert it with `?` and print it with its causes; the commands that keep going past a failed
//! item print it with [`Error::chain_text`].
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays `step: cause` with the whole cause chain and
//! exposes no [`std::error::Error::source`], so a caller walking the chain prints each cause
//! exactly once; a wrapped library error displays and chains exactly as that library's error does.

use std::fmt::Display;

/// Why a blueprint compiler command could not finish.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted, printed as `step: cause` with every cause below.
    #[error("{context}: {}", cause.chain_text())]
    Context {
        /// The step that failed, as the command output names it.
        context: String,
        /// The failure underneath.
        cause: Box<Error>,
    },
    /// A file or a folder could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document could not be parsed or written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A pak archive or a loose game file could not be read.
    #[error(transparent)]
    Pak(#[from] enfusion_pak::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// An integer field of a dump, a model or an argument does not parse.
    #[error(transparent)]
    ParseInteger(#[from] std::num::ParseIntError),
    /// A decimal field of a dump or an argument does not parse.
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
    /// A count or an index does not fit its on-disk width.
    #[error(transparent)]
    IntegerWidth(#[from] std::num::TryFromIntError),
    /// An occlusion sidecar does not parse.
    #[error(transparent)]
    Sidecar(#[from] spatial_indexes::prelude::BvhParseError),
    /// A prefab descriptor does not project into its archive form.
    #[error(transparent)]
    ArchiveProjection(#[from] world_line_of_sight::ArchiveProjectionError),
    /// A contract schema does not compile into a validator.
    #[error(transparent)]
    Schema(#[from] jsonschema::ValidationError<'static>),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub(crate) fn message(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// The message followed by every cause underneath, each after `: `, the way a chained
    /// `{:#}` print names them.
    #[must_use]
    pub fn chain_text(&self) -> String {
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
        $crate::error::Error::message(format!($($argument)*))
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
