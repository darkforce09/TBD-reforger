//! Why a value breaks the fleet wire contract.
//!
//! **Role:** the crate's one error type and its `Result` alias.
//! **Position:** returned by
//! [`crate::machine_credential_format::check_machine_credential_format`]; the host agent reports
//! it as the reason its credential file is refused.
//! **Signals & state:** none; plain data.
//! **Invariants:** an error never carries the secret it refuses, so printing it leaks nothing.

/// Why a value breaks the fleet wire contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The text is not `tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>`.
    #[error("a machine credential reads tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>")]
    MalformedMachineCredential,
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
