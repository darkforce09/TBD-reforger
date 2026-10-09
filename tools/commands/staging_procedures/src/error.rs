//! Why a staging command, a host read or a procedure step failed.
//!
//! **Role:** the crate's [`Error`] and [`Result`], the crate-private `ensure!`, `bail!` and
//! `refusal!` macros that build an [`Error::Refused`], and the crate-private [`ResultExt`] that
//! puts the step that failed in front of a failure.
//! **Position:** returned by every fallible call of the crate; the xtask binary prints it with
//! `xtask: {error:#}` and exits 1, and the procedures print it into a step's verdict, a case's
//! failure, a manifest's `unavailable` value or an action list line.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant's text is complete on its own: an [`Error::Context`] displays
//! `<step>: <cause>` and exposes no source, so a caller that prints the chain (`{error:#}`) and
//! one that prints the text (`{error}`) print the same line, the text the receipts, action lists
//! and operator prompts have always carried; a wrapped error of another crate keeps its own text.

use std::fmt::Display;

/// Why a staging command, a host read or a procedure step failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A rule a setting, a plan, a host answer or a committed file breaks; the text is the whole
    /// message.
    #[error("{0}")]
    Refused(String),
    /// A failure with the step that hit it in front of it.
    #[error("{context}: {cause}")]
    Context {
        /// The step that failed.
        context: String,
        /// The failure itself; its text follows the step.
        cause: Box<Error>,
    },
    /// A file, folder or output stream operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A host answer, a receipt part or a committed file is not the JSON it must be.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A committed file or a host answer is not UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    /// A number in a host answer or a setting is out of range or not a number.
    #[error(transparent)]
    Number(#[from] std::num::ParseIntError),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// A `deploy.env` setting is missing or invalid.
    #[error(transparent)]
    Settings(#[from] deploy_settings::Error),
    /// The fleet layout the deploy shares could not be read.
    #[error(transparent)]
    Deployment(#[from] deployment::Error),
    /// The load plan or report could not be built, encoded or decoded.
    #[error(transparent)]
    LoadPlan(#[from] staging_load_plan::Error),
    /// A raw artifact could not be digested.
    #[error(transparent)]
    Digest(#[from] content_digest::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Puts the step that failed in front of a failure, or turns a missing value into a refusal.
pub(crate) trait ResultExt<T> {
    /// The failure behind `context`.
    fn context(self, context: impl Display) -> Result<T>;
    /// The failure behind the context `make` builds, built only on failure.
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
        self.ok_or_else(|| Error::Refused(context.to_string()))
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.ok_or_else(|| Error::Refused(make().to_string()))
    }
}

/// An [`Error::Refused`] with the formatted text.
macro_rules! refusal {
    ($($message:tt)+) => {
        $crate::error::Error::Refused(format!($($message)+))
    };
}

/// Returns an [`Error::Refused`] with the formatted text.
macro_rules! bail {
    ($($message:tt)+) => {
        return Err($crate::error::refusal!($($message)+))
    };
}

/// Returns an [`Error::Refused`] with the formatted text unless `condition` holds.
macro_rules! ensure {
    ($condition:expr, $($message:tt)+) => {
        if !$condition {
            $crate::error::bail!($($message)+);
        }
    };
}

pub(crate) use {bail, ensure, refusal};
