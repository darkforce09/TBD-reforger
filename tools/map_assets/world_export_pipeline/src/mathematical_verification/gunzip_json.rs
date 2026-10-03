//! **Role:** reading one gzip JSON artifact.
//! **Position:** the gates of `verify-phase` read artifacts through it.
//! **Signals & state:** none.
//! **Invariants:** a read or parse failure names the file.

use super::*;

/// Reads a gzip JSON file.
pub fn gunzip_json(p: &Path) -> Result<Value> {
    let raw = gunzip(&std::fs::read(p).with_context(|| p.display().to_string())?)?;
    Ok(serde_json::from_slice(&raw)?)
}
