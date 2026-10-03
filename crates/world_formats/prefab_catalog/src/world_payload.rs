//! Decoding of a served world payload: gzip-or-plain JSON, and the one error every world parse
//! answers with.
//!
//! **Role:** turns the bytes of a fetched or read world file (prefab catalogue, chunk, roads,
//! regions) into a `serde_json::Value`, and names every way a world parse fails.
//! **Position:** the bottom of `prefab_catalog`; read by [`crate::prefab_tables`], the map
//! engine's world store and chunk ingest, the developer tools' export and raster pipelines, and the
//! world tests.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** a payload that starts with the gzip magic `1f 8b` is inflated before the JSON
//! parse and anything else is parsed as it is; a failure is a [`WorldError`], never a panic or a
//! partial value.

use std::io::Read;

use serde_json::Value;

use crate::error::InvalidPrefabId;

/// A world-parse failure (gunzip, JSON, a binary archive, or a manifest missing the object-export paths).
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    /// The payload starts with the gzip magic but does not inflate; carries the inflate error.
    #[error("world: gzip inflate failed: {0}")]
    Gzip(String),

    /// The (inflated) payload is not a JSON document; carries the parser's message.
    #[error("world: json parse failed: {0}")]
    Json(String),

    /// The terrain manifest has no `objects` block with string `prefabsPath` and `chunksPath`.
    #[error("world: manifest missing objects/prefabsPath/chunksPath")]
    Manifest,

    /// A binary archive failed to load; the message is its `BinaryError`.
    #[error("world: binary archive failed to load: {0}")]
    Archive(String),

    /// A zero-length payload. Named rather than folded into [`WorldError::Json`] because the format sniff cannot even *classify* an empty buffer, and "json parse failed: EOF while parsing a value" is a misleading thing to say about a file that was never fetched.
    #[error("world: empty payload — no format to sniff")]
    EmptyPayload,

    /// A catalogue row's `prefabId` is not a whole number in `0..=u32::MAX`.
    #[error("world: {0}")]
    InvalidPrefabId(#[from] InvalidPrefabId),
}

/// Decode a world payload: inflate it when it starts with the gzip magic `1f 8b`, then parse the
/// JSON document.
pub fn bytes_to_json(bytes: &[u8]) -> Result<Value, WorldError> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut inflated = Vec::new();
        decoder
            .read_to_end(&mut inflated)
            .map_err(|e| WorldError::Gzip(e.to_string()))?;
        serde_json::from_slice(&inflated).map_err(|e| WorldError::Json(e.to_string()))
    } else {
        serde_json::from_slice(bytes).map_err(|e| WorldError::Json(e.to_string()))
    }
}
