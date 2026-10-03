//! Why an API readiness call failed.
//!
//! **Role:** the crate's [`Error`] and [`Result`], the crate-private `ensure!`, `bail!` and
//! `refusal!` macros that build a [`Error::Refused`], and the crate-private [`ResultExt`] that puts
//! the step that failed in front of a failure.
//! **Position:** returned by the register reader, the evidence judge, the fingerprints, the
//! evidence writer and the staging recorder; `xtask` prints it through `anyhow` and the judge
//! prints it into a check's verdict.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant's text is complete on its own: a [`Error::Context`] displays
//! `<step>: <cause>` and exposes no source, so a caller that prints the chain (`{error:#}`) and
//! one that prints the text (`{error}`) print the same line, the text the checks have always
//! printed.

use std::fmt::Display;

/// Why an API readiness call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A rule the register, a receipt, a log value or the environment breaks; the text is the
    /// whole message.
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
    /// A file or folder operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A register, receipt or property record is not the JSON it must be.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A case pattern is not a valid regular expression.
    #[error(transparent)]
    Pattern(#[from] regex::Error),
    /// A number in the environment is out of range or not a number.
    #[error(transparent)]
    Number(#[from] std::num::ParseIntError),
    /// An evidence file name holds a NUL byte.
    #[error(transparent)]
    NulByte(#[from] std::ffi::NulError),
    /// A fingerprint input could not be digested.
    #[error(transparent)]
    Digest(#[from] content_digest::Error),
}

/// The result of an API readiness call.
pub type Result<T> = std::result::Result<T, Error>;

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
