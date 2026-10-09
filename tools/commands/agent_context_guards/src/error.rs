//! Why the filtered command runner failed.
//!
//! **Role:** the crate's [`Error`] and [`Result`].
//! **Position:** returned by [`crate::run_filtered_command`]; `xtask` prints it through `anyhow`
//! as `xtask: <cause>`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a command that runs and exits non-zero is an exit code, never an error; an
//! error is only a command that could not be run at all. The tool-call guard has no error: it
//! fails open.

/// Why the filtered command runner failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `sh` could not be spawned or its output could not be collected.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The crate's result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;
