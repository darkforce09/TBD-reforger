//! Why a world-export stage or gate could not run.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private [`ResultExt`] that adds
//! a context line to a failure or turns a missing value into a refusal, and the crate-private
//! `refuse!` macro that returns a refusal built like `format!`.
//! **Position:** returned by every fallible entry of the crate; the `world` command line prints it
//! as `world: <message>` and exits 1, except a [`Error::Stop`], whose message it prints bare and
//! whose exit code it returns. The map raster pipeline and the map verification convert it with
//! `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays `context: cause` and exposes no source, so its
//! text is the whole chain and the cause is printed exactly once; a wrapped error of another crate
//! displays its own text and exposes its own source; a refusal's text is the operator's diagnosis,
//! fixed per site.

use std::fmt::Display;

/// Why a world-export stage or gate could not run.
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
    /// A stage stopped with its diagnosis and the exit code the `world` binary returns.
    #[error("{message}")]
    Stop {
        /// The process exit code.
        code: u8,
        /// The diagnosis, printed bare on standard error.
        message: String,
    },
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document could not be parsed or written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// The game's archives could not be opened or read.
    #[error(transparent)]
    Pak(#[from] enfusion_pak::Error),
    /// A `cargo` child process could not be run, or died without an exit code.
    #[error(transparent)]
    NotRun(#[from] verification_core::NotRun),
    /// A prefab catalogue row's `prefabId` is not a whole number in `0..=u32::MAX`.
    #[error(transparent)]
    InvalidPrefabId(#[from] prefab_catalog::InvalidPrefabId),
    /// A PNG could not be encoded.
    #[error(transparent)]
    PngEncoding(#[from] png::EncodingError),
    /// A PNG could not be decoded.
    #[error(transparent)]
    PngDecoding(#[from] png::DecodingError),
    /// A walked file does not lie under the folder it was walked from.
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    /// A count or coordinate does not fit the integer width its format stores.
    #[error(transparent)]
    IntegerWidth(#[from] std::num::TryFromIntError),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// The message without its cause: the step that failed, for a report line that names a failure
    /// in one short clause.
    pub(crate) fn headline(&self) -> String {
        match self {
            Error::Context { context, .. } => context.clone(),
            other => other.to_string(),
        }
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
