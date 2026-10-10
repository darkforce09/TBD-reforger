//! The world tooling's virtual file system over an Enfusion install.
//!
//! **Role:** [`PakVfs`] finds the game's `addons/`, opens it under the world policy, and exposes
//! inflated and raw entry reads with the diagnostic metadata the decoders need.
//! **Position:** over [`crate::PakSet`]; called by the world export and map raster pipelines and
//! by `enf extract` and `enf dump-entry`.
//! **Signals & state:** reads `ENFUSION_GAME_PATH` and `HOME` in [`PakVfs::open_default`].
//! **Invariants:** paths keep their case; a malformed archive is skipped, never fatal; the
//! default game folder is `ENFUSION_GAME_PATH`, else the enfusion-mcp cache under `$HOME`.

use std::path::{Path, PathBuf};

use crate::read_policy::ReadPolicy;
use crate::{AssetSource, Error, PakEntry, PakIndex, PakSet, Result};

/// The archives of one game install, opened under the world policy.
pub struct PakVfs {
    archives: PakSet,
}

impl PakVfs {
    /// Open every archive in `<game_path>/addons/`.
    pub fn open(game_path: &Path) -> Result<Self> {
        let addons = game_path.join("addons");
        if !addons.exists() {
            return Err(Error::NoAddons {
                game: game_path.to_path_buf(),
            });
        }
        Ok(Self {
            archives: PakSet::from_dir_with_policy(&addons, ReadPolicy::World)?,
        })
    }

    /// Open the game folder named by `ENFUSION_GAME_PATH`, else `default_game_root` (the caller's
    /// checkout-local MCP game root).
    pub fn open_default(default_game_root: &Path) -> Result<Self> {
        let game = game_root(default_game_root);
        Self::open(&game).map_err(|error| Error::NoPakVfs {
            reason: error.headline(),
        })
    }

    fn entry(&self, path: &str) -> Result<(&PakIndex, &PakEntry)> {
        self.archives
            .entry(path)
            .ok_or_else(|| Error::FileNotInPak {
                path: path.to_string(),
            })
    }

    /// Whether an archive holds `path`.
    pub fn exists(&self, path: &str) -> bool {
        self.archives.exists(path)
    }

    /// The inflated content of `path`.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let (archive, entry) = self.entry(path)?;
        archive.read_with_policy(entry, ReadPolicy::World)
    }

    /// The stored bytes of `path` and its decompressed length from the directory.
    pub fn read_raw(&self, path: &str) -> Result<(Vec<u8>, u32)> {
        let (archive, entry) = self.entry(path)?;
        Ok((archive.read_raw(entry)?, entry.decompressed_len))
    }

    /// The `DATA` chunk offset of the archive holding `path`.
    pub fn entry_data_start(&self, path: &str) -> Option<u64> {
        self.archives
            .entry(path)
            .map(|(archive, _)| archive.data_start)
    }

    /// The method tag and compressed flag of `path`'s record.
    pub fn entry_method(&self, path: &str) -> Option<([u8; 6], bool)> {
        self.archives
            .entry(path)
            .map(|(_, entry)| (entry.method, entry.compressed))
    }

    /// Every virtual path across the archives, unordered.
    pub fn all_file_paths(&self) -> Vec<&str> {
        self.archives.all_file_paths()
    }
}

/// The game folder named by `ENFUSION_GAME_PATH` when set and non-empty, else `default_game_root`.
pub(crate) fn game_root(default_game_root: &Path) -> PathBuf {
    std::env::var_os("ENFUSION_GAME_PATH")
        .filter(|value| !value.is_empty())
        .map_or_else(|| default_game_root.to_path_buf(), PathBuf::from)
}

#[cfg(test)]
#[path = "tests/world_source.rs"]
mod tests;
