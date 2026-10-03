//! Why a map raster subcommand could not finish.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private `ResultExt` that adds
//! the step that failed to an error or turns a missing value into a refusal, and the crate-private
//! macros `refusal!` (a refusal built like `format!`) and `bail!` (return one).
//! **Position:** returned by every fallible function of the crate; the `map` command line prints
//! it as `map: <message>` with every cause underneath ([`Error::chain_text`]) and exits 1.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays `step: cause` with the whole cause chain and
//! exposes no [`std::error::Error::source`], so a caller walking the chain prints each cause
//! exactly once; a wrapped library error displays and chains exactly as that library's error does.

use std::fmt::Display;

/// Why a map raster subcommand could not finish.
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
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryLayout(#[from] repository_layout::Error),
    /// A pak archive or a loose game file could not be read.
    #[error(transparent)]
    Pak(#[from] enfusion_pak::Error),
    /// A world-export decoder or number formatter failed.
    #[error(transparent)]
    WorldExport(#[from] world_export_pipeline::Error),
    /// A binary archive could not be serialised or read back.
    #[error(transparent)]
    Archive(#[from] world_file_formats::archives::codec::BinaryError),
    /// An image could not be opened, decoded or encoded.
    #[error(transparent)]
    Image(#[from] image::ImageError),
    /// A PNG could not be encoded.
    #[error(transparent)]
    PngEncoding(#[from] png::EncodingError),
    /// A PNG could not be decoded.
    #[error(transparent)]
    PngDecoding(#[from] png::DecodingError),
    /// A lossless WebP image could not be encoded.
    #[error(transparent)]
    WebpEncoding(#[from] image_webp::EncodingError),
    /// A count or a coordinate does not fit the integer width its format stores.
    #[error(transparent)]
    IntegerWidth(#[from] std::num::TryFromIntError),
    /// A raster value does not parse as an integer.
    #[error(transparent)]
    ParseInteger(#[from] std::num::ParseIntError),
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

/// The result of a fallible call of this crate; the error type defaults to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A refusal ([`Error::Message`]) whose text is built like `format!`.
macro_rules! refusal {
    ($($argument:tt)*) => {
        $crate::error::Error::msg(format!($($argument)*))
    };
}
pub(crate) use refusal;

/// Returns a refusal ([`Error::Message`]) whose text is built like `format!`.
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
