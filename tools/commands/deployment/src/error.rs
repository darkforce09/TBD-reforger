//! Why a website deploy, a staging deploy or the staging compose-path check failed.
//!
//! **Role:** the crate's [`Error`] and its [`Result`] alias.
//! **Position:** returned by every public entry of the crate; the xtask binary prints it with
//! `xtask: {error:#}` and exits 1, and prints a `deploy db` operator stop
//! ([`database_operations::Error::Stop`]) bare. A deploy's or a gate's verdict is its exit code,
//! not an error: an error means the step could not run.
//! **Signals & state:** none; plain data.
//! **Invariants:** a wrapped error of another crate keeps its own text and source chain.

/// Why a website deploy, a staging deploy or the staging compose-path check failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// A regular expression built from a scanned name does not compile.
    #[error(transparent)]
    Pattern(#[from] regex::Error),
    /// A `deploy db` verb failed or stopped.
    #[error(transparent)]
    Database(#[from] database_operations::Error),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// The `deploy db` operator stop this error carries, or `None`.
    pub fn database_stop_report(&self) -> Option<&str> {
        match self {
            Error::Database(error) => error.stop_report(),
            _ => None,
        }
    }
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
