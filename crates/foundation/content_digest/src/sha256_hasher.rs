//! Incremental SHA-256 with length framing.
//!
//! **Role:** [`Sha256Hasher`] feeds bytes to one SHA-256 in pieces: raw ([`Sha256Hasher::update`]),
//! framed by their 8-byte little-endian length ([`Sha256Hasher::update_length_framed`]) or as a
//! whole file framed the same way ([`Sha256Hasher::update_file_length_framed`]), and spells the
//! result as lowercase hex ([`Sha256Hasher::finalize_hex`]).
//! **Position:** called by the repository tooling that fingerprints many inputs into one digest
//! (export policy digests, staging workload digests).
//! **Signals & state:** the running SHA-256 state, owned by the caller's hasher value.
//! **Invariants:** a framed field hashes as its length (`u64`, little-endian) followed by its
//! bytes, so two different field sequences never feed the same byte stream; a framed file hashes
//! exactly like its bytes framed in memory, and a file whose length changes while it is read is an
//! error, never a digest.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::lowercase_hex::lowercase_hex;

/// Size of the buffer a file is streamed through.
const FILE_READ_BUFFER_BYTES: usize = 64 * 1024;

/// One SHA-256 computation fed in pieces; [`Sha256Hasher::finalize_hex`] ends it.
#[derive(Clone, Default)]
pub struct Sha256Hasher {
    state: Sha256,
}

impl Sha256Hasher {
    /// A hasher that has seen no bytes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds `bytes` as they are, with no framing.
    pub fn update(&mut self, bytes: impl AsRef<[u8]>) {
        self.state.update(bytes.as_ref());
    }

    /// Feeds the 8-byte little-endian length of `bytes`, then `bytes`.
    pub fn update_length_framed(&mut self, bytes: impl AsRef<[u8]>) {
        let bytes = bytes.as_ref();
        self.state.update((bytes.len() as u64).to_le_bytes());
        self.state.update(bytes);
    }

    /// Feeds a file framed like [`Sha256Hasher::update_length_framed`] of its bytes: the length
    /// its metadata records (8 bytes, little-endian), then the bytes streamed from disk.
    ///
    /// # Errors
    ///
    /// [`Error::FileUnreadable`] when the file cannot be opened, inspected or read;
    /// [`Error::FileLengthChanged`] when the bytes read differ in number from the recorded length.
    /// The hasher has then seen a partial input and must be discarded.
    pub fn update_file_length_framed(&mut self, path: &Path) -> Result<()> {
        let unreadable = |source| Error::FileUnreadable {
            path: path.to_path_buf(),
            source,
        };
        let mut file = std::fs::File::open(path).map_err(unreadable)?;
        let recorded = file.metadata().map_err(unreadable)?.len();
        self.state.update(recorded.to_le_bytes());
        let mut read = 0_u64;
        let mut buffer = vec![0_u8; FILE_READ_BUFFER_BYTES];
        loop {
            let count = file.read(&mut buffer).map_err(unreadable)?;
            if count == 0 {
                break;
            }
            read += count as u64;
            self.state.update(&buffer[..count]);
        }
        if read != recorded {
            return Err(Error::FileLengthChanged {
                path: path.to_path_buf(),
                recorded,
                read,
            });
        }
        Ok(())
    }

    /// Ends the computation and returns the digest as lowercase hex.
    pub fn finalize_hex(self) -> String {
        lowercase_hex(&self.state.finalize())
    }
}

#[cfg(test)]
#[path = "tests/sha256_hasher.rs"]
mod tests;
