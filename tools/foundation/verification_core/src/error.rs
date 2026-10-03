//! Why a verification primitive could not answer.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the fallible primitives return their own cause — [`crate::NotRun`] from the
//! readers, the scans and the lock, `regex::Error` from [`crate::Pattern::regex`]; a caller that
//! gathers several of them converts each into an [`Error`] with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** an input that was never examined is an [`Error::NotRun`], never a pass.

use crate::verdict::NotRun;

/// Why a verification primitive could not answer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The check could not run: a target is missing or unreadable, a tool is absent or failed,
    /// a child died on a signal, or a deadline passed.
    #[error(transparent)]
    NotRun(#[from] NotRun),
    /// A search pattern does not compile.
    #[error(transparent)]
    InvalidPattern(#[from] regex::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
