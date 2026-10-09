//! Lowercase hex SHA-256 and SHA-384 digests.
//!
//! **Role:** the one place the workspace turns bytes into a SHA-2 hex string: whole inputs
//! ([`sha256_hex`], [`sha384_hex`]), whole files ([`sha384_hex_of_file`], the checksum `sqlx`
//! stores for a migration) and an incremental SHA-256 ([`Sha256Hasher`]) that frames each field
//! with its length so concatenated inputs cannot collide.
//! **Position:** foundation tier over `sha2`. The repository tooling (migration checksum repair,
//! export manifests, staging receipts) calls it; nothing it calls is a
//! workspace crate.
//! **Signals & state:** none; pure functions, plus a hasher value its caller owns.
//! **Invariants:** every digest is lowercase hexadecimal, two characters per byte, with no
//! separator, byte-identical to `sha256sum` and `sha384sum`; a file is hashed as its exact bytes.

mod error;
mod hex_digests;
mod lowercase_hex;
pub mod prelude;
mod sha256_hasher;

pub use error::{Error, Result};
pub use hex_digests::{sha256_hex, sha384_hex, sha384_hex_of_file};
pub use sha256_hasher::Sha256Hasher;
