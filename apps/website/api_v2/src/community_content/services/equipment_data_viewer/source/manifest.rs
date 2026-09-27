//! Published-bundle identity, integrity and contained filesystem paths.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default = "diagnostic_kind")]
    pub dataset_kind: String,
    pub schema_version: u32,
    pub generation_id: String,
    pub resource_count: u64,
    pub files: BTreeMap<String, FileDigest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedPointer {
    #[serde(default = "diagnostic_kind")]
    pub dataset_kind: String,
    pub schema_version: u32,
    pub generation_id: String,
    pub directory: String,
    pub manifest_sha256: String,
}

fn diagnostic_kind() -> String {
    "diagnostic".into()
}

pub fn supported(kind: &str, version: u32) -> bool {
    matches!((kind, version), ("diagnostic", 2) | ("gameplay", 1))
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResourceEntry {
    pub resource_id: String,
    pub resource_name: String,
    pub record_file: String,
    pub source_file: String,
    pub domains: Vec<String>,
}

pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn safe_child(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty() && !relative.contains('\\'),
        "invalid dataset path"
    );
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        let Component::Normal(segment) = part else {
            anyhow::bail!("dataset path escapes its root")
        };
        path.push(segment);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(!meta.file_type().is_symlink(), "symlink inside dataset");
        }
    }
    Ok(path)
}

pub fn read(root: &Path, relative: &str) -> Result<Vec<u8>> {
    let path = safe_child(root, relative)?;
    let metadata = fs::metadata(&path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= 128 * 1024 * 1024,
        "invalid or oversized dataset document"
    );
    fs::read(path).context("read dataset document")
}

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
