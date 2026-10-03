//! Durable activation keeps previous data intact through partial writes.
use crate::error::{Result, ensure};
use api_identifiers::EquipmentGenerationId;
use content_digest::Sha256Hasher;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

/// Writes `bytes` to a new file at `path` and syncs it; an existing file is an error.
pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// The lowercase hex SHA-256 of the file at `path`, streamed in 64 KiB reads; the same digest
/// [`crate::source::manifest::digest`] gives its bytes.
pub fn file_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 65536];
    let mut digest = Sha256Hasher::new();
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest.finalize_hex())
}

/// Syncs every folder under `root`, and `root` itself, so a rename into it is durable.
pub fn sync_tree(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            sync_tree(&entry.path())?;
        }
    }
    File::open(root)?.sync_all()?;
    Ok(())
}

/// Renames the finished `staging` folder to `destination`, which must not exist yet, and syncs
/// its parent.
pub fn finish_directory(staging: &Path, destination: &Path) -> Result<()> {
    ensure!(!destination.exists(), "destination already exists");
    let parent = destination.parent().expect("destination parent");
    fs::create_dir_all(parent)?;
    fs::rename(staging, destination)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

/// Points `root`'s `current.json` at generation `id` through a synced temporary file and a rename;
/// when the folder sync fails, the previous pointer is put back.
pub fn activate(root: &Path, id: &EquipmentGenerationId, manifest_hash: &str) -> Result<()> {
    let current = root.join("current.json");
    let temp = root.join(".current.json.tmp");
    if temp.exists() {
        fs::remove_file(&temp)?;
    }
    let previous = fs::read(&current).ok();
    let value = serde_json::json!({"generation_id":id,"index_version":super::super::INDEX_VERSION,"manifest_sha256":manifest_hash});
    write(&temp, &serde_json::to_vec_pretty(&value)?)?;
    fs::rename(&temp, &current)?;
    if let Err(error) = File::open(root).and_then(|f| f.sync_all()) {
        if let Some(bytes) = previous {
            write(&temp, &bytes)?;
            fs::rename(&temp, &current)?;
        } else {
            fs::remove_file(&current)?;
        }
        return Err(error.into());
    }
    Ok(())
}
