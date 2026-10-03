//! Why no checkout root was found.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** returned by [`crate::find_repository_root`] and
//! [`crate::find_repository_root_from`]; a caller on `anyhow` converts it with `?`.
//! **Signals & state:** none; plain data.
//! **Invariants:** a failed walk names the folder it started from and the marker it looked for.

use std::path::PathBuf;

/// Why no checkout root was found.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The working directory, where [`crate::find_repository_root`] starts, could not be read.
    #[error("the working directory could not be read: {0}")]
    CurrentDirectory(#[source] std::io::Error),
    /// No folder from `start` up to the filesystem root holds [`crate::ROOT_MARKER`].
    #[error(
        "could not find the repository root: no folder from {} upward holds {}",
        start.display(),
        crate::ROOT_MARKER
    )]
    RootMarkerNotFound {
        /// The folder the walk started from.
        start: PathBuf,
    },
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
