//! Opaque-token primitives: random generation, SHA-256 storage hashing, and constant-time
//! comparison.

use rand::{Rng, RngExt};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Cryptographically random hex string with `n_bytes` of entropy (OAuth state,
/// opaque refresh tokens).
pub fn random_token(n_bytes: usize) -> String {
    let mut b = vec![0u8; n_bytes];
    rand::rng().fill_bytes(&mut b);
    hex::encode(b)
}

/// Hex SHA-256 of a token — refresh tokens are stored hashed so a DB leak does not
/// expose usable credentials.
pub fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

/// Compare two strings without leaking timing information. Unequal lengths compare `false`.
pub fn constant_time_equal(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Zero-padded random decimal code of `digits` length, e.g. `numeric_code(6)` →
/// `"042199"`. Backs the Arma identity link code.
pub fn numeric_code(digits: u32) -> String {
    let upper = 10u64.pow(digits);
    let n = rand::rng().random_range(0..upper);
    format!("{n:0width$}", width = digits as usize)
}

#[cfg(test)]
#[path = "tests/token_hashing.rs"]
mod tests;
