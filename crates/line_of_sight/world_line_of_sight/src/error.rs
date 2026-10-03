//! Why the prefab occluder library refuses an archive or a descriptor.
//!
//! **Role:** the crate's one error type and its `Result` alias, wrapping the building archive's
//! read error and the descriptor projection error.
//! **Position:** converted into with `?` from [`crate::occluder_library`]'s archive read and
//! descriptor projection.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its error transparently: the message and the source are the
//! wrapped error's own.

use world_file_formats::archives::codec::BinaryError;

use crate::occluder_library::ArchiveProjectionError;

/// Why the world line of sight refuses its input.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The building archive does not validate or is of another schema version.
    #[error(transparent)]
    Archive(#[from] BinaryError),

    /// A descriptor cannot be projected into the building archive.
    #[error(transparent)]
    ArchiveProjection(#[from] ArchiveProjectionError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
