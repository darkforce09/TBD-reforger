//! Why a map asset gate could not run to its verdict.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private `ResultExt` that adds
//! the step that failed to an error or turns a missing value into a refusal, and the
//! crate-private macros `refusal!` (a refusal built like `format!`) and `bail!` (return one).
//! **Position:** returned by every fallible gate of the crate; the CI task catalogue's map asset
//! checks and the `cargo xtask map world-los` adapter wrap it and print it with its causes.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays only its step and exposes the failure underneath
//! as its [`std::error::Error::source`], so a plain print names the step alone and a caller
//! walking the chain ([`Error::chain_text`], `{:#}` through a chain printer) prints `step: cause`
//! with every cause exactly once; a wrapped library error displays and chains exactly as that
//! library's error does. A gate's own findings are never errors: they print as `FAIL` lines and
//! set the exit code.

use std::fmt::Display;

/// Why a map asset gate could not run to its verdict.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted; the failure underneath is the source.
    #[error("{context}")]
    Context {
        /// The step that failed, as the gate output names it.
        context: String,
        /// The failure underneath.
        #[source]
        cause: Box<Error>,
    },
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON document could not be parsed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A command-line count does not parse.
    #[error(transparent)]
    ParseInteger(#[from] std::num::ParseIntError),
    /// A command-line fraction does not parse.
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
    /// An elevation raster could not be decoded.
    #[error(transparent)]
    Png(#[from] png::DecodingError),
    /// The prefab catalogue holds a prefab id outside its range.
    #[error(transparent)]
    PrefabId(#[from] prefab_catalog::InvalidPrefabId),
    /// A chunk container does not parse or frames another chunk.
    #[error(transparent)]
    ChunkContainer(#[from] world_chunks::chunk_container::ChunkBinError),
    /// An occlusion sidecar does not parse.
    #[error(transparent)]
    Sidecar(#[from] spatial_indexes::prelude::BvhParseError),
    /// The world export pipeline could not emit a chunk container.
    #[error(transparent)]
    WorldExport(#[from] world_export_pipeline::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub(crate) fn msg(text: impl Into<String>) -> Self {
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
