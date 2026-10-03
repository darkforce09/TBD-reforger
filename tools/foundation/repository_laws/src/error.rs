//! Why a repository law could not judge the tree.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** every law returns its cause as a [`verification_core::NotRun`]; a caller that
//! runs several laws converts each into an [`Error`] with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a law whose input is missing or unreadable is an [`Error`], never a pass.

use verification_core::NotRun;

/// Why a repository law could not judge the tree.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An input the law needs is missing or unreadable.
    #[error(transparent)]
    NotRun(#[from] NotRun),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
