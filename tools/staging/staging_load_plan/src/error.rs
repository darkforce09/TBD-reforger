//! Why a workload could not be read, a plan was refused, or a source address is not this
//! machine's.
//!
//! - **Role:** the crate's [`Error`] and its [`Result`] alias.
//! - **Position:** returned by the workload decoders, the plan and workload checks, the request
//!   catalog's compilation, the process-boundary codec, [`crate::reachable_member_accounts`] and
//!   [`crate::verify_source_addresses`]; the load generator wraps it, and a command on `anyhow`
//!   converts it with `?` and prints its text as it stands.
//! - **Signals & state:** none; plain data.
//! - **Invariants:** a refusal names the first field, template, origin, address or event that
//!   fails, and each message embeds the text of the failure beneath it, so the printed line is
//!   the whole explanation; no message echoes a credential (an origin is quoted only once it is
//!   known to carry none).

use std::io;
use std::net::IpAddr;
use std::path::PathBuf;

/// Why a workload or a plan cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A workload document is not JSON of the workload's shape.
    #[error("decoding the load workload: {error}")]
    WorkloadUndecodable {
        /// The decoding failure; its text is part of this error's own message.
        error: serde_json::Error,
    },
    /// A workload file could not be read.
    #[error("reading the load workload {}: {error}", path.display())]
    WorkloadFileUnreadable {
        /// The workload file.
        path: PathBuf,
        /// The read failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// A workload file is not JSON of the workload's shape.
    #[error("decoding the load workload {}: {error}", path.display())]
    WorkloadFileUndecodable {
        /// The workload file.
        path: PathBuf,
        /// The decoding failure; its text is part of this error's own message.
        error: serde_json::Error,
    },
    /// A number, template, origin, address or fixture event of the workload or the run is
    /// refused.
    #[error("{reason}")]
    Refused {
        /// What is refused and why.
        reason: String,
    },
    /// A plan or a report cannot be encoded as JSON.
    #[error("encoding the load {what}: {error}")]
    Unencodable {
        /// `plan` or `report`.
        what: &'static str,
        /// The encoding failure; its text is part of this error's own message.
        error: serde_json::Error,
    },
    /// A plan or a report crossing the process boundary is not JSON of its shape.
    #[error("decoding the load {what}: {error}")]
    Undecodable {
        /// `plan` or `report`.
        what: &'static str,
        /// The decoding failure; its text is part of this error's own message.
        error: serde_json::Error,
    },
    /// The operating system refuses to bind a port on a source address.
    #[error("source address {address} is not assigned to this machine: {error}")]
    SourceAddressNotAssigned {
        /// The refused address.
        address: IpAddr,
        /// The bind failure; its text is part of this error's own message.
        error: io::Error,
    },
}

impl Error {
    /// A refusal with the text `reason`.
    pub(crate) fn refused(reason: impl Into<String>) -> Self {
        Self::Refused {
            reason: reason.into(),
        }
    }
}

/// The result of reading, checking or measuring a load plan.
pub type Result<T> = std::result::Result<T, Error>;

/// Return [`Error::Refused`] with the formatted reason unless the condition holds.
macro_rules! refuse_unless {
    ($condition:expr, $($reason:tt)+) => {
        let holds: bool = $condition;
        if !holds {
            return Err($crate::error::Error::refused(format!($($reason)+)));
        }
    };
}

pub(crate) use refuse_unless;
