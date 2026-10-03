//! The limits of a secret file: the file a machine credential or an RCON password is handed
//! over in.
//!
//! **Role:** the size and permission rules the writer of a secret file creates it with and the
//! reader of one refuses it without.
//! **Position:** the staging fixtures tool of the API writes credential files under these rules;
//! the host agent reads its credential and RCON password files under the same rules.
//! **Signals & state:** none; constants.
//! **Invariants:** a secret file is created with [`SECRET_FILE_MODE`], which carries none of
//! [`SHARED_PERMISSION_BITS`], so every file a writer creates passes the reader's permission
//! check; a machine credential (102 bytes) fits well within [`SECRET_FILE_MAX_BYTES`].

/// The largest secret file either side accepts, in bytes.
pub const SECRET_FILE_MAX_BYTES: u64 = 4096;

/// The mode a secret file is created with: read and write for the owner only.
pub const SECRET_FILE_MODE: u32 = 0o600;

/// The group and other permission bits; a secret file or secrets directory carrying any of them
/// is refused.
pub const SHARED_PERMISSION_BITS: u32 = 0o077;

#[cfg(test)]
#[path = "tests/secret_file_limits.rs"]
mod tests;
