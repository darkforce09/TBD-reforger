//! Why a child process gave no answer.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** [`crate::Run`], [`crate::which`], [`crate::retry`] and [`crate::wait_for`]
//! return their cause as a [`verification_core::NotRun`]; a caller that gathers several failures
//! converts it into an [`Error`] with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a child that was signalled, timed out, or never spawned is an [`Error`], never
//! an exit code.

use verification_core::NotRun;

/// Why a child process gave no answer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The program is absent, failed to spawn or to be reaped, died on a signal, or ran out of
    /// time.
    #[error(transparent)]
    NotRun(#[from] NotRun),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
