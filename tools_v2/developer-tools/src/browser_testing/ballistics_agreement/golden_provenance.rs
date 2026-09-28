//! Proof that the catalog the browser is served is the committed catalog.
//!
//! **Role:** reads the two captured API goldens the gate answers the bench's catalog reads with
//! — the public list and the version's document — and proves them the committed catalog: the
//! list names the version with the committed file's SHA-256, and the document decodes to the
//! same catalog as the committed file.
//! **Position:** called by [`super::run`] before the browser starts; the [`ServedGoldens`] it
//! returns are the exact bytes `super::browser_session` fulfils the two reads with. The goldens
//! live in `apps/website/frontend/tests/fixtures/api/`, captured from the real API.
//! **Signals & state:** none; reads files.
//! **Invariants:** a missing, unreadable or mismatching golden is a failure with its cause, never
//! a skip; the SHA-256 is of the committed file's bytes, as the API computes `catalog_sha256`
//! at upload; the document is compared decoded, because the API re-serialises the stored
//! document.

use std::path::Path;

use sha2::{Digest, Sha256};
use website_map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;

/// The captured list golden's file name.
pub const LIST_GOLDEN: &str = "GET__ballistics-catalogs.json";

/// The two golden bodies the browser is served.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServedGoldens {
    /// Body of `GET /api/v1/ballistics-catalogs`.
    pub list_body: Vec<u8>,
    /// Body of `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`.
    pub document_body: Vec<u8>,
}

/// The captured document golden's file name for one catalog version.
pub fn document_golden(catalog_id: &str, catalog_version: u32) -> String {
    format!("GET__ballistics-catalogs__{catalog_id}__versions__{catalog_version}.json")
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Reads the goldens in `fixtures_dir` and proves them the committed catalog whose file bytes
/// are `committed_bytes` and whose decoding is `committed`.
///
/// # Errors
///
/// A sentence naming the golden and what is wrong with it.
pub fn check_served_goldens(
    fixtures_dir: &Path,
    committed_bytes: &[u8],
    committed: &BallisticsCatalog,
) -> Result<ServedGoldens, String> {
    let list_path = fixtures_dir.join(LIST_GOLDEN);
    let document_path = fixtures_dir.join(document_golden(
        &committed.catalog_id,
        committed.catalog_version,
    ));
    let list_body = read_golden(&list_path)?;
    let document_body = read_golden(&document_path)?;

    let list: serde_json::Value = serde_json::from_slice(&list_body)
        .map_err(|error| format!("{} is not JSON: {error}", list_path.display()))?;
    let rows = list["data"]
        .as_array()
        .ok_or_else(|| format!("{} has no `data` array", list_path.display()))?;
    let row = rows
        .iter()
        .find(|row| {
            row["catalog_id"] == committed.catalog_id.as_str()
                && row["catalog_version"] == committed.catalog_version
        })
        .ok_or_else(|| {
            format!(
                "{} does not list {} v{}",
                list_path.display(),
                committed.catalog_id,
                committed.catalog_version
            )
        })?;
    let committed_sha256 = sha256_hex(committed_bytes);
    let served_sha256 = row["catalog_sha256"].as_str().unwrap_or_default();
    if served_sha256 != committed_sha256 {
        return Err(format!(
            "{} names catalog_sha256 {served_sha256}, the committed catalog hashes to {committed_sha256}",
            list_path.display()
        ));
    }

    let served = BallisticsCatalog::from_json_slice(&document_body)
        .map_err(|error| format!("{} does not decode: {error}", document_path.display()))?;
    if &served != committed {
        return Err(format!(
            "{} decodes to another catalog than the committed one",
            document_path.display()
        ));
    }
    Ok(ServedGoldens {
        list_body,
        document_body,
    })
}

fn read_golden(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| {
        format!(
            "the API golden {} is unreadable ({error}); capture it from the real API",
            path.display()
        )
    })
}

#[cfg(test)]
#[path = "../tests/ballistics_agreement/golden_provenance.rs"]
mod tests;
