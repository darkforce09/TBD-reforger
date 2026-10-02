//! Durable activation keeps previous data intact through partial writes.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn file_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 65536];
    let mut digest = Sha256::new();
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

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

pub fn finish_directory(staging: &Path, destination: &Path) -> Result<()> {
    ensure!(!destination.exists(), "destination already exists");
    let parent = destination.parent().expect("destination parent");
    fs::create_dir_all(parent)?;
    fs::rename(staging, destination)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn activate(root: &Path, id: &str, manifest_hash: &str) -> Result<()> {
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
