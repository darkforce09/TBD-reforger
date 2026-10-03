//! Why a load run could not start or finish, and why the `staging-load` executable wrote no
//! report.
//!
//! - **Role:** the crate's [`Error`] and its [`Result`] alias.
//! - **Position:** returned by [`crate::run`] and by the executable's command line, which prints
//!   it to standard error and exits 1; a plan's own refusals arrive wrapped from
//!   `staging_load_plan`.
//! - **Signals & state:** none; plain data.
//! - **Invariants:** a failed request is never an error (the report counts it); no message echoes
//!   a token, since an account file refusal names the file, the entry and the problem only; each
//!   message embeds the text of the failure beneath it, so the printed line is the whole
//!   explanation.

use std::io;
use std::net::IpAddr;
use std::path::PathBuf;

/// Why a load run or the executable around it stopped.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plan, its addresses or its encoding are refused.
    #[error(transparent)]
    Plan(#[from] staging_load_plan::Error),
    /// The account file could not be read.
    #[error("reading the account file {}: {error}", path.display())]
    AccountFileUnreadable {
        /// The account file the plan names.
        path: PathBuf,
        /// The read failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// The account file is not an account list of the workload's size.
    #[error("{reason}")]
    AccountFileRefused {
        /// What is wrong with the file; it never echoes a token.
        reason: String,
    },
    /// The HTTP client of one source address could not be built.
    #[error("building the HTTP client bound to {address}: {error}")]
    HttpClientNotBuilt {
        /// The source address the client binds to.
        address: IpAddr,
        /// The build failure; its text is part of this error's own message.
        error: reqwest::Error,
    },
    /// The run's runtime could not be built.
    #[error("building the load generation runtime: {error}")]
    RuntimeNotBuilt {
        /// The build failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// A virtual client's task panicked or was cancelled.
    #[error("a virtual client task stopped abnormally: {error}")]
    ClientTaskStopped {
        /// The join failure; its text is part of this error's own message.
        error: tokio::task::JoinError,
    },
    /// The plan file named on the command line could not be read.
    #[error("reading the load plan {}: {error}", path.display())]
    PlanFileUnreadable {
        /// The plan file.
        path: PathBuf,
        /// The read failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// The plan could not be read from standard input.
    #[error("reading the load plan from standard input: {error}")]
    PlanInputUnreadable {
        /// The read failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// The report file named on the command line could not be written.
    #[error("writing the load report {}: {error}", path.display())]
    ReportFileUnwritable {
        /// The report file.
        path: PathBuf,
        /// The write failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// The report could not be written to standard output.
    #[error("writing the load report to standard output: {error}")]
    ReportOutputUnwritable {
        /// The write failure; its text is part of this error's own message.
        error: io::Error,
    },
}

impl Error {
    /// An account file refusal with the text `reason`.
    pub(crate) fn account_file_refused(reason: impl Into<String>) -> Self {
        Self::AccountFileRefused {
            reason: reason.into(),
        }
    }
}

/// The result of a load run or of the executable's command line.
pub type Result<T> = std::result::Result<T, Error>;

/// Return [`Error::AccountFileRefused`] with the formatted reason unless the condition holds.
macro_rules! refuse_account_file_unless {
    ($condition:expr, $($reason:tt)+) => {
        let holds: bool = $condition;
        if !holds {
            return Err($crate::error::Error::account_file_refused(format!($($reason)+)));
        }
    };
}

pub(crate) use refuse_account_file_unless;
