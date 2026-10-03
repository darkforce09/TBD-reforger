//! Published-bundle identity, integrity and contained filesystem paths.
use crate::error::{Error, Result, ensure};
use api_identifiers::{EquipmentGenerationId, EquipmentResourceId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The size and SHA-256 a manifest records for one file.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    /// The file's size in bytes.
    pub bytes: u64,
    /// The lowercase hex SHA-256 of the file's bytes.
    pub sha256: String,
}

/// The manifest of one published generation: what it holds and every file's digest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The dataset kind, `gameplay` or `diagnostic` (the default).
    #[serde(default = "diagnostic_kind")]
    pub dataset_kind: String,
    /// The manifest format version.
    pub schema_version: u32,
    /// The generation the manifest describes.
    pub generation_id: EquipmentGenerationId,
    /// How many resources the generation holds.
    pub resource_count: u64,
    /// Every file of the generation, by its path relative to the generation's root.
    pub files: BTreeMap<String, FileDigest>,
}

/// The `current.json` pointer the export publishes: which generation is complete and where.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedPointer {
    /// The dataset kind, `gameplay` or `diagnostic` (the default).
    #[serde(default = "diagnostic_kind")]
    pub dataset_kind: String,
    /// The pointer format version.
    pub schema_version: u32,
    /// The generation the pointer names.
    pub generation_id: EquipmentGenerationId,
    /// The generation's folder, relative to the export source.
    pub directory: String,
    /// The lowercase hex SHA-256 of the generation's `manifest.json`.
    pub manifest_sha256: String,
}

fn diagnostic_kind() -> String {
    "diagnostic".into()
}

/// Whether this importer reads a `kind` dataset of schema `version`: diagnostic 2 or gameplay 1.
pub fn supported(kind: &str, version: u32) -> bool {
    matches!((kind, version), ("diagnostic", 2) | ("gameplay", 1))
}

/// One resource a generation lists, with the files that hold it.
#[derive(Clone, Debug, Deserialize)]
pub struct ResourceEntry {
    /// The resource's identity.
    pub resource_id: EquipmentResourceId,
    /// The resource's file name in the game data.
    pub resource_name: String,
    /// The file holding the resource's record.
    pub record_file: String,
    /// The file holding the resource's source snapshot.
    pub source_file: String,
    /// The equipment domains the resource belongs to.
    pub domains: Vec<String>,
}

/// The lowercase hex SHA-256 of `bytes`, the spelling every manifest digest uses.
pub fn digest(bytes: &[u8]) -> String {
    content_digest::sha256_hex(bytes)
}

/// `relative` joined under `root`, refused when it is empty, holds a backslash, leaves `root`
/// through any component other than a plain name, or passes a symlink.
pub fn safe_child(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty() && !relative.contains('\\'),
        "invalid dataset path"
    );
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        let Component::Normal(segment) = part else {
            return Err(Error::check_failed("dataset path escapes its root"));
        };
        path.push(segment);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(!meta.file_type().is_symlink(), "symlink inside dataset");
        }
    }
    Ok(path)
}

/// The bytes of the regular file `relative` under `root` ([`safe_child`]), refused above 128 MiB.
pub fn read(root: &Path, relative: &str) -> Result<Vec<u8>> {
    let path = safe_child(root, relative)?;
    let metadata = fs::metadata(&path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= 128 * 1024 * 1024,
        "invalid or oversized dataset document"
    );
    fs::read(path).map_err(Error::DocumentRead)
}

/// Every regular file under `root`, by its path relative to `root`; a symlink or any other
/// non-file fails the walk.
pub fn inventory(root: &Path) -> Result<BTreeSet<String>> {
    fn visit(root: &Path, dir: &Path, found: &mut BTreeSet<String>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            ensure!(!kind.is_symlink(), "symlink inside dataset");
            if kind.is_dir() {
                visit(root, &entry.path(), found)?;
            } else {
                ensure!(kind.is_file(), "non-file in dataset");
                found.insert(
                    entry
                        .path()
                        .strip_prefix(root)?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        Ok(())
    }
    let mut found = BTreeSet::new();
    visit(root, root, &mut found)?;
    Ok(found)
}
