//! Why a repository check could not run to its verdict.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** every check returns its exit status as `Ok`; an [`Error`] is a check that could
//! not start or read what it needs, which the xtask binary prints and exits 1 on.
//! **Signals & state:** none; plain data.
//! **Invariants:** a check that could not read its input never returns a passing status; each
//! variant keeps the text the gate printed for that cause.

use verification_core::NotRun;

/// Why a repository check could not run to its verdict.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No checkout root was found from the working directory.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_layout::Error),
    /// A file or folder the check reads could not be read.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A program or input the check needs is missing, failed to start, or was signalled.
    #[error(transparent)]
    NotRun(#[from] NotRun),
    /// `git ls-files -z`, the tracked-file walk of the language bans, could not run.
    #[error("git ls-files -z: {source}")]
    TrackedFileListing {
        /// Why `git` did not run.
        source: NotRun,
    },
}

/// The result of a fallible call of this crate; a module that judges with another error type
/// names it as the second parameter.
pub type Result<T, E = Error> = std::result::Result<T, E>;
