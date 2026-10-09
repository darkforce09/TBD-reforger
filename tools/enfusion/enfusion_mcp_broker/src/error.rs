//! Why the broker's server child could not be started.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the broker's child start returns it; the daemon prints it after
//! `mcp-daemon: child init failed: ` and exits 2, or logs it and stops when a restart fails.
//! **Signals & state:** none; plain data.
//! **Invariants:** a wrapped cause prints exactly as the cause itself does.

/// Why the broker's server child could not be started.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No checkout to resolve the server command against.
    #[error(transparent)]
    RepositoryRoot(#[from] repository_root::Error),
    /// The server could not be spawned, or its stdin refused the initialisation.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The spawned server came without one of its piped streams.
    #[error("child {stream} is not piped")]
    MissingPipe {
        /// `stdin`, `stdout` or `stderr`.
        stream: &'static str,
    },
    /// The server exited before it answered the initialisation.
    #[error("child exited during init")]
    ExitedDuringInit,
    /// The server did not answer the initialisation within the call timeout.
    #[error("init timeout")]
    InitTimeout,
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
