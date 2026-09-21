//! SHA-384 over file content, in the hex spelling `sqlx` stores.
//!
//! `sqlx` records `Sha384::digest(migration_sql)` in `_sqlx_migrations.checksum` and compares it
//! byte-for-byte on every boot, so reading or repairing that column means hashing a file exactly
//! as `sqlx` would: the whole file, comments and trailing newline included, with no normalisation.

use sha2::{Digest, Sha384};

/// Lowercase hex SHA-384 of `bytes`.
pub fn sha384_hex(bytes: &[u8]) -> String {
    let digest = Sha384::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Lowercase hex SHA-384 of a file's bytes, or `None` when it cannot be read.
pub fn sha384_hex_of_file(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| sha384_hex(&bytes))
}

#[cfg(test)]
#[path = "tests/content_digest/tests.rs"]
mod tests;
