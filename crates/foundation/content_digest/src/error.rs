//! Why a file cannot be fed to a hasher.
//!
//! **Role:** the crate's one error type and its `Result` alias.
//! **Position:** returned by [`crate::Sha256Hasher::update_file_length_framed`]; the caller
//! reports it as the reason a fingerprint cannot be computed.
//! **Signals & state:** none; plain data.
//! **Invariants:** an unreadable or changing file is an [`Error`], never a digest of partial bytes.

use std::path::PathBuf;

/// Why a file's bytes cannot be hashed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Opening the file, reading its metadata or reading its bytes failed.
    #[error("cannot read {path} for hashing: {source}")]
    FileUnreadable {
        /// The file that was being hashed.
        path: PathBuf,
        /// The operating system's reason.
        source: std::io::Error,
    },
    /// The file holds a different number of bytes than its metadata recorded when hashing began.
    #[error("{path} changed length while being hashed: {recorded} bytes recorded, {read} read")]
    FileLengthChanged {
        /// The file that was being hashed.
        path: PathBuf,
        /// The length the file's metadata recorded before reading.
        recorded: u64,
        /// The number of bytes actually read.
        read: u64,
    },
}

/// The crate's `Result`, failing with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
