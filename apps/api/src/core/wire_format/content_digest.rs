//! Content digests of JSON wire values: canonical JSON and its SHA-256.
//!
//! **Role:** turns a JSON value into the one byte string every writer of the same value produces,
//! and digests it, so a digest identifies content rather than formatting.
//! **Position:** shared primitive of [`crate::core::wire_format`]; the mission artifact store and
//! the match-telemetry ingest digest their inputs through it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** object keys are sorted explicitly at every depth (serde_json's
//! `preserve_order` feature is unified into this build, so a `Map` would otherwise keep insertion
//! order); arrays keep their order; the output is compact serde_json text; the digest is
//! lowercase hexadecimal SHA-256.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Canonical JSON (object keys sorted at every depth, no whitespace) for digests of structured
/// inputs.
pub fn canonical_json(value: &Value) -> String {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|key| (key.clone(), sorted(&map[key])))
                        .collect(),
                )
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    serde_json::to_string(&sorted(value)).unwrap_or_default()
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Lowercase hexadecimal SHA-256 of the canonical JSON of `value`.
pub fn canonical_sha256(value: &Value) -> String {
    sha256_hex(canonical_json(value).as_bytes())
}

#[cfg(test)]
#[path = "tests/content_digest.rs"]
mod tests;
