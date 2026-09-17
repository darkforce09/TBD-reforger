//! World-tooling facade preserving game-root discovery and diagnostic access.
use super::{AssetSource, PakEntry, PakIndex, PakSet, ReadPolicy};
use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};

pub struct PakVfs {
    archives: PakSet,
}

impl PakVfs {
    pub fn open(game_path: &Path) -> Result<Self> {
        let addons = game_path.join("addons");
        if !addons.exists() {
            bail!("no addons/ under {}", game_path.display());
        }
        Ok(Self {
            archives: PakSet::from_dir_with_policy(&addons, ReadPolicy::World)?,
        })
    }

    pub fn open_default() -> Result<Self> {
        let game = std::env::var("ENFUSION_GAME_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".cache/enfusion-mcp-root")
            });
        Self::open(&game).map_err(|error| anyhow!("No pak VFS ({error}) — set ENFUSION_GAME_PATH"))
    }

    fn entry(&self, path: &str) -> Result<(&PakIndex, &PakEntry)> {
        self.archives
            .entry(path)
            .ok_or_else(|| anyhow!("File not found in pak: {path}"))
    }

    pub fn exists(&self, path: &str) -> bool {
        self.archives.exists(path)
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let (archive, entry) = self.entry(path)?;
        archive.read_with_policy(entry, ReadPolicy::World)
    }

    pub fn read_raw(&self, path: &str) -> Result<(Vec<u8>, u32)> {
        let (archive, entry) = self.entry(path)?;
        Ok((archive.read_raw(entry)?, entry.decompressed_len))
    }

    pub fn entry_data_start(&self, path: &str) -> Option<u64> {
        self.archives
            .entry(path)
            .map(|(archive, _)| archive.data_start)
    }

    pub fn entry_method(&self, path: &str) -> Option<([u8; 6], bool)> {
        self.archives
            .entry(path)
            .map(|(_, entry)| (entry.method, entry.compressed))
    }

    pub fn all_file_paths(&self) -> Vec<&str> {
        self.archives.all_file_paths()
    }
}

#[cfg(test)]
#[path = "tests/world_source.rs"]
mod tests;
