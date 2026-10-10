//! Why a `ttm` call produced no usable answer.
//!
//! **Role:** the crate's [`Error`] and its [`Result`] alias: the command did not run, the ticket
//! manager refused, or its output broke the JSON contract.
//! **Position:** returned by every [`crate::TicketManager`] call and by
//! [`crate::parse_document`]; the command crates wrap it in their own error enums and print its
//! text as it stands.
//! **Signals & state:** none; plain data.
//! **Invariants:** a refusal keeps the ticket manager's own error kind and message, so a caller can
//! tell "not found" from "conflict" without parsing prose; a missing binary is never reported as a
//! refusal, and an unreadable answer is never reported as an empty one.

use verification_core::NotRun;

/// The error kind `ttm` reports for a reference or a wave lock it does not know.
pub const NOT_FOUND_KIND: &str = "not_found";

/// Why a `ttm` call produced no usable answer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The command did not run to an exit code: the binary is missing, or it died on a signal or
    /// a deadline.
    #[error(
        "`{command}` did not run: {source} (install the central ticket manager's `ttm`, or point \
         TBD_TTM_BIN at it)"
    )]
    NotRun {
        /// The command line, as an operator would type it.
        command: String,
        /// Why the child did not run to an exit code.
        #[source]
        source: NotRun,
    },
    /// The ticket manager ran and refused the request.
    #[error("`{command}` refused ({kind}): {message}")]
    Refused {
        /// The command line, as an operator would type it.
        command: String,
        /// The ticket manager's error kind (`not_found`, `conflict`, `validation`, …), or
        /// `exit <code>` when it printed no error document.
        kind: String,
        /// The ticket manager's message, or its stderr when it printed no error document.
        message: String,
        /// The candidates of an ambiguous reference; empty otherwise.
        candidates: Vec<String>,
    },
    /// The output is not the JSON document the contract promises for this command.
    #[error("`{command}` broke the ttm JSON contract: {detail}")]
    Contract {
        /// The command line, as an operator would type it.
        command: String,
        /// What was wrong with the output.
        detail: String,
    },
}

impl Error {
    /// Whether the ticket manager refused because it knows no such ticket, wave or lock.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::Refused { kind, .. } if kind == NOT_FOUND_KIND)
    }
}

/// The result of a `ttm` call.
pub type Result<T, E = Error> = std::result::Result<T, E>;
