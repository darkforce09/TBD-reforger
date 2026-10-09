//! Why a mod script check could not run to its verdict.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** every check returns its exit status as `Ok`; an [`Error`] is a check that could
//! not start or read what it needs, which the xtask binary prints and exits 1 on.
//! **Signals & state:** none; plain data.
//! **Invariants:** a check that could not read its input never returns a passing status.

/// Why a mod script check could not run to its verdict.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No checkout root was found from the working directory.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// A file or folder the check reads or writes could not be accessed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A pattern the check builds at run time did not compile.
    #[error(transparent)]
    Regex(#[from] regex::Error),
}

/// The result of a fallible call of this crate; a module that judges with another error type
/// names it as the second parameter.
pub type Result<T, E = Error> = std::result::Result<T, E>;
