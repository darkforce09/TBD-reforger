//! How a `staging-fixtures` run ends when it does not complete: refused, or failed.
//!
//! **Role:** the one failure type the argument parser, the guards and every subcommand return,
//! and the exit code of each class.
//!
//! **Position:** produced anywhere in the tool; `main.rs` prints it to stderr and exits with
//! [`ToolFailure::exit_code`].
//!
//! **Signals & state:** none; plain values.
//!
//! **Invariants:** a refusal (exit 2) means the run wrote nothing; a failure (exit 1) means the run
//! rolled back its transaction and removed the files it had written, or its message names what
//! remains. No message carries a secret: messages name paths, ids and flags, never file contents or
//! connection strings.

use std::fmt;
use std::process::ExitCode;

use website_api::core::error_handling::api_error::ApiError;

/// The exit code of a run that failed after it started writing.
const FAILED_EXIT_CODE: u8 = 1;
/// The exit code of a run that the arguments or a guard stopped before any write.
const REFUSED_EXIT_CODE: u8 = 2;

/// Why a run stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolFailure {
    /// The arguments or a guard stopped the run; nothing was written.
    Refused(String),
    /// An operation failed; the run's writes were undone, or the message names what remains.
    Failed(String),
}

impl ToolFailure {
    /// A refusal carrying `reason`.
    pub(crate) fn refused(reason: impl Into<String>) -> Self {
        Self::Refused(reason.into())
    }

    /// A failure carrying `reason`.
    pub(crate) fn failed(reason: impl Into<String>) -> Self {
        Self::Failed(reason.into())
    }

    /// The process exit code of this class: 2 for a refusal, 1 for a failure.
    pub(crate) fn exit_code(&self) -> ExitCode {
        match self {
            Self::Refused(_) => ExitCode::from(REFUSED_EXIT_CODE),
            Self::Failed(_) => ExitCode::from(FAILED_EXIT_CODE),
        }
    }
}

impl fmt::Display for ToolFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(reason) => write!(formatter, "refused: {reason}"),
            Self::Failed(reason) => write!(formatter, "failed: {reason}"),
        }
    }
}

impl From<sqlx::Error> for ToolFailure {
    fn from(error: sqlx::Error) -> Self {
        Self::failed(format!("database error: {error}"))
    }
}

/// A 4xx from a service is a rule the request broke, which the run's rollback leaves unwritten; a
/// 5xx is a failure whose cause the service logged to stderr.
impl From<ApiError> for ToolFailure {
    fn from(error: ApiError) -> Self {
        if error.status.is_client_error() {
            Self::refused(error.message)
        } else {
            Self::failed(error.message)
        }
    }
}
