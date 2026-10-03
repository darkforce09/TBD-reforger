//! The failures of the equipment datasets: imports, index builds and generation-pinned reads.
//!
//! **Role:** one typed error for every fallible call of the dataset service, with the helpers its
//! modules use to refuse a failed check (`ensure!`) or an absent value
//! ([`Required::required`]).
//! **Position:** `api_equipment_datasets`; the debug viewer handlers of `api_community_content`
//! answer any of these as 400 with the error's message, and the equipment export watcher records
//! the message with its causes as the import progress.
//! **Signals & state:** none; plain values.
//! **Invariants:** a failed check carries the message that names it; a wrapped library failure
//! renders as that library renders it; the three steps that name what failed
//! ([`Error::ExportSourceUnavailable`], [`Error::ImportActive`], [`Error::DocumentRead`]) keep
//! the underlying failure as their source.

/// A failed equipment dataset import, index build or read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A dataset, query or import check failed; the message names the check.
    #[error("{0}")]
    CheckFailed(String),
    /// The configured export source directory cannot be resolved.
    #[error("export source directory is unavailable")]
    ExportSourceUnavailable(#[source] std::io::Error),
    /// Another process holds the import lock of the data directory.
    #[error("another equipment import is active")]
    ImportActive(#[source] std::fs::TryLockError),
    /// A dataset document could not be read after its path and size were checked.
    #[error("read dataset document")]
    DocumentRead(#[source] std::io::Error),
    /// A filesystem operation failed.
    #[error(transparent)]
    Filesystem(#[from] std::io::Error),
    /// A JSON document or value did not decode or encode.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The SQLite navigation index failed.
    #[error(transparent)]
    Index(#[from] sqlx::Error),
    /// A blocking task of the import or of a read did not complete.
    #[error(transparent)]
    BackgroundTask(#[from] tokio::task::JoinError),
    /// The read permit could not be acquired.
    #[error(transparent)]
    ReaderPermit(#[from] tokio::sync::AcquireError),
    /// A cursor or an array index is not a number.
    #[error(transparent)]
    Number(#[from] std::num::ParseIntError),
    /// A dataset file does not sit under the root it was listed from.
    #[error(transparent)]
    PathOutsideRoot(#[from] std::path::StripPrefixError),
}

/// The result of an equipment dataset call.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// A failed check named by `message`.
    pub fn check_failed(message: impl Into<String>) -> Self {
        Self::CheckFailed(message.into())
    }
}

/// Turns an absent value into [`Error::CheckFailed`] naming what was required.
pub trait Required<T> {
    /// The value, or [`Error::CheckFailed`] carrying `message` when it is absent.
    fn required(self, message: &str) -> Result<T>;
}

impl<T> Required<T> for Option<T> {
    fn required(self, message: &str) -> Result<T> {
        self.ok_or_else(|| Error::check_failed(message))
    }
}

/// Returns [`Error::CheckFailed`] with the formatted message from the enclosing function or
/// closure unless the condition holds.
macro_rules! ensure {
    ($condition:expr, $($message:tt)+) => {
        if !$condition {
            return Err(
                $crate::error::Error::CheckFailed(
                    format!($($message)+),
                ),
            );
        }
    };
}

pub(crate) use ensure;
