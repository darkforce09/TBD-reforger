//! Token helpers — Rust port of `internal/auth/tokens.go`.

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

/// Compare two strings without leaking timing information. Like Go's
/// `subtle.ConstantTimeCompare`, unequal lengths compare `false`.
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
mod tests {
    use super::*;

    #[test]
    fn numeric_code_is_zero_padded_digits() {
        for _ in 0..100 {
            let c = numeric_code(6);
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn constant_time_equal_matches_go_semantics() {
        assert!(constant_time_equal("abc", "abc"));
        assert!(!constant_time_equal("abc", "abd"));
        assert!(!constant_time_equal("abc", "ab")); // unequal length -> false
        assert!(!constant_time_equal("", "x"));
        assert!(constant_time_equal("", ""));
    }

    #[test]
    fn hash_token_is_sha256_hex() {
        assert_eq!(
            hash_token(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_token("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn random_token_hex_length_and_uniqueness() {
        assert_eq!(random_token(16).len(), 32);
        assert_eq!(random_token(32).len(), 64);
        assert_ne!(random_token(16), random_token(16));
    }
}
