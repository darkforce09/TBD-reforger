//! Lowercase hex SHA-256 and SHA-384 of whole inputs and files.
//!
//! **Role:** the one-shot digests: [`sha256_hex`] of bytes, [`sha384_hex`] of bytes and
//! [`sha384_hex_of_file`], the SHA-384 hex `sqlx` stores for a migration.
//! **Position:** called by the repository tooling (`cargo xtask db repair-migration-checksum`,
//! export manifests, staging receipts); spells its output through [`crate::lowercase_hex`].
//! **Signals & state:** none; pure functions and one file read.
//! **Invariants:** `sqlx` records `Sha384::digest(migration_sql)` in
//! `_sqlx_migrations.checksum` and compares it byte-for-byte on every boot, so a file is hashed
//! exactly as `sqlx` would: the whole file, comments and trailing newline included, with no
//! normalisation.

use sha2::{Digest, Sha256, Sha384};

use crate::lowercase_hex::lowercase_hex;

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    lowercase_hex(&Sha256::digest(bytes))
}

/// Lowercase hex SHA-384 of `bytes`.
pub fn sha384_hex(bytes: &[u8]) -> String {
    lowercase_hex(&Sha384::digest(bytes))
}

/// Lowercase hex SHA-384 of a file's bytes, or `None` when it cannot be read.
pub fn sha384_hex_of_file(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| sha384_hex(&bytes))
}

#[cfg(test)]
#[path = "tests/hex_digests.rs"]
mod tests;
