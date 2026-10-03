//! Why a CI task step, an `mk` recipe or a CI workflow check could not run.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias and [`cause_chain`], the one rendering of
//! an error with its causes.
//! **Position:** returned by every fallible public entry of the crate; the xtask binary prints it
//! with `xtask: {error:#}` and exits 1, and prints a database operator stop that the database lane
//! raised inside `mk rust-ci` bare ([`Error::database_stop_report`]). The task runner prints an
//! in-process step's error through [`cause_chain`], the same text. A gate's verdict is its exit
//! code, not an error: an error means the step could not run.
//! **Signals & state:** none; plain data.
//! **Invariants:** a wrapped error of another crate keeps its own text and source chain; a map
//! asset check's failure keeps the cause chain the developer tools library gave it.

/// Why a CI task step, an `mk` recipe or a CI workflow check could not run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A map asset check of the developer tools library failed: its whole text, causes included.
    #[error("{0}")]
    MapAssetCheck(String),
    /// A file could not be read, written or removed, or a child could not be started.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The checkout root could not be found.
    #[error(transparent)]
    RepositoryLayout(#[from] repository_layout::Error),
    /// A language ban, workspace law, route-tag or engine-layer check could not run.
    #[error(transparent)]
    RepositoryChecks(#[from] repository_checks::Error),
    /// A mod script check could not run.
    #[error(transparent)]
    ModScriptChecks(#[from] mod_script_checks::Error),
    /// A contract codegen or schema gate could not run.
    #[error(transparent)]
    SchemaTooling(#[from] schema_tooling::Error),
    /// The database lane's suite or a database source check could not run, or stopped.
    #[error(transparent)]
    DatabaseOperations(#[from] database_operations::Error),
    /// The staging compose-path check could not run.
    #[error(transparent)]
    Deployment(#[from] deployment::Error),
    /// `cargo test -p <package>` could not be started.
    #[error("start `cargo test -p {package}`")]
    CargoTestStart {
        /// The package whose test run could not start.
        package: String,
        /// Why the child could not be started or reaped.
        #[source]
        source: verification_core::NotRun,
    },
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// A map asset check's failure, rendered with `{:#}` so its causes stay in the text.
    pub(crate) fn map_asset_check(failure: impl std::fmt::Display) -> Self {
        Error::MapAssetCheck(format!("{failure:#}"))
    }

    /// The database operator stop this error carries, or `None`.
    pub fn database_stop_report(&self) -> Option<&str> {
        match self {
            Error::DatabaseOperations(error) => error.stop_report(),
            _ => None,
        }
    }
}

/// `error` and every cause behind it, joined by `": "`: the text `xtask: {error:#}` prints.
pub fn cause_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

/// The result of a fallible call of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
