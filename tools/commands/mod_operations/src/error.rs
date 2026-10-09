//! Why a mod command could not do its work.
//!
//! **Role:** the crate's [`Error`] and [`Result`], the crate-private `ensure!`, `bail!` and
//! `refusal!` macros that build an [`Error::Refused`], and the crate-private [`ResultExt`] that
//! puts the step that failed in front of a failure.
//! **Position:** returned by every fallible call of the crate; the xtask binary prints it with
//! `xtask: {error:#}` and exits 1, and the gates fold it into a verdict line, a validation report
//! entry or an environment failure.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant's text is complete on its own: an [`Error::Context`] displays
//! `<step>: <cause>` and exposes no source, so a caller that prints the chain (`{error:#}`) and
//! one that prints the text (`{error}`) print the same line, the one the commands have always
//! printed; a wrapped error of another crate keeps its own text. A gate's verdict is its exit
//! code, never an error.

use std::fmt::Display;

use verification_core::NotRun;

/// Why a mod command could not do its work.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A rule an export, a policy, an API answer or a setting breaks; the text is the whole
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
    /// A document is not the JSON it must be, or a value could not be encoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A published file could not be locked.
    #[error(transparent)]
    Lock(#[from] std::fs::TryLockError),
    /// A folder walk over an export or a policy tree failed.
    #[error(transparent)]
    Walk(#[from] walkdir::Error),
    /// A walked path does not lie under the folder it was walked from.
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    /// A number in a program's answer is out of range or not a number.
    #[error(transparent)]
    Number(#[from] std::num::ParseIntError),
    /// A committed JSON Schema could not be compiled.
    #[error(transparent)]
    Schema(Box<jsonschema::ValidationError<'static>>),
    /// A program could not be started, or died on a signal or a deadline.
    #[error(transparent)]
    ProgramNotRun(#[from] NotRun),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// The remote log verdict could not run.
    #[error(transparent)]
    RemoteDebugging(#[from] remote_debugging::Error),
    /// A spawn determinism or spawn verification run could not run.
    #[error(transparent)]
    ModScriptChecks(#[from] mod_script_checks::Error),
    /// The staging host check or the MCP game root could not run.
    #[error(transparent)]
    WorkstationSetup(#[from] workstation_setup::Error),
    /// The milestone announcement could not be seeded.
    #[error(transparent)]
    DatabaseOperations(#[from] database_operations::Error),
    /// A slice worktree step of the mod wave driver could not run.
    #[error(transparent)]
    PlatformExecution(#[from] platform_execution::Error),
}

impl From<jsonschema::ValidationError<'static>> for Error {
    fn from(error: jsonschema::ValidationError<'static>) -> Self {
        Error::Schema(Box::new(error))
    }
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
