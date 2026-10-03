//! Sorted archive merging and policy-specific virtual path resolution.
//!
//! **Role:** declares [`AssetSource`], the read interface every source serves, and [`PakSet`],
//! the merged directories of a folder of archives, with [`normalize_path`], the blueprint lookup
//! key.
//! **Position:** over [`crate::PakIndex`]; [`crate::PakVfs`] wraps a world-policy set, and the
//! blueprint compiler reads a blueprint-policy set directly.
//! **Signals & state:** a [`PakSet`] owns its indexes and its lookup table, built once.
//! **Invariants:** archives open in sorted name order and the first one holding a path wins;
//! under the blueprint policy a malformed archive fails the whole set, under the world policy it
//! is skipped with a message on stderr.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::read_policy::ReadPolicy;
use crate::{Error, PakEntry, PakIndex, Result};

/// A virtual game-file source, including archives and loose extracted directories.
pub trait AssetSource {
    /// The content of the file at `rel_path`.
    fn read(&self, rel_path: &str) -> Result<Vec<u8>>;
    /// Whether this source holds a file at `rel_path`.
    fn exists(&self, rel_path: &str) -> bool;
    /// The content of the file at `rel_path` as text, invalid UTF-8 replaced.
    fn read_text(&self, rel_path: &str) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.read(rel_path)?).into_owned())
    }
}

/// Blueprint lookup keys fold ASCII case and normalize path separators.
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

fn lookup_key(path: &str, policy: ReadPolicy) -> String {
    if policy == ReadPolicy::Blueprint {
        return normalize_path(path);
    }
    let normalized = path.replace('\\', "/");
    let mut out = String::new();
    let mut previous_slash = false;
    for ch in normalized.trim_matches('/').chars() {
        if ch != '/' || !previous_slash {
            out.push(ch);
        }
        previous_slash = ch == '/';
    }
    out
}

/// The merged directories of every `.pak` in one folder.
#[derive(Debug)]
pub struct PakSet {
    pub(crate) paks: Vec<PakIndex>,
    lookup: HashMap<String, (usize, usize)>,
    policy: ReadPolicy,
}

impl PakSet {
    /// The `addons/` folder of the enfusion-mcp game cache under `$HOME`, when `HOME` is set.
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/enfusion-mcp-root/addons"))
    }

    /// Open every `.pak` in `dir` under the blueprint policy.
    pub fn from_dir(dir: &Path) -> Result<Self> {
        Self::from_dir_with_policy(dir, ReadPolicy::Blueprint)
    }

    pub(crate) fn from_dir_with_policy(dir: &Path, policy: ReadPolicy) -> Result<Self> {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|cause| Error::ReadDirectory {
                directory: dir.to_path_buf(),
                cause,
            })?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pak"))
            })
            .collect();
        paths.sort();
        if paths.is_empty() {
            return Err(Error::NoPakFiles {
                directory: dir.to_path_buf(),
            });
        }
        let mut set = Self {
            paks: Vec::new(),
            lookup: HashMap::new(),
            policy,
        };
        for path in paths {
            match PakIndex::open_with_policy(&path, policy) {
                Ok(index) => set.push(index),
                Err(error) if policy == ReadPolicy::World => {
                    eprintln!(
                        "pak: failed to parse {}: {}",
                        path.display(),
                        error.headline()
                    );
                }
                Err(error) => return Err(error),
            }
        }
        Ok(set)
    }

    /// Add one archive. Later archives never shadow an existing virtual path.
    pub fn push(&mut self, index: PakIndex) {
        let pak = self.paks.len();
        for (entry, item) in index.entries.iter().enumerate() {
            self.lookup
                .entry(lookup_key(&item.path, self.policy))
                .or_insert((pak, entry));
        }
        self.paks.push(index);
    }

    /// The number of archives in the set.
    pub fn pak_count(&self) -> usize {
        self.paks.len()
    }
    /// The number of distinct virtual paths across the set.
    pub fn file_count(&self) -> usize {
        self.lookup.len()
    }
    /// The record of the archive that holds `path`.
    pub fn find(&self, path: &str) -> Option<&PakEntry> {
        self.entry(path).map(|(_, entry)| entry)
    }

    pub(crate) fn entry(&self, path: &str) -> Option<(&PakIndex, &PakEntry)> {
        let (pak, entry) = *self.lookup.get(&lookup_key(path, self.policy))?;
        Some((&self.paks[pak], &self.paks[pak].entries[entry]))
    }

    pub(crate) fn all_file_paths(&self) -> Vec<&str> {
        self.lookup.keys().map(String::as_str).collect()
    }

    /// Every stored path whose lookup key starts with `prefix`'s, sorted.
    pub fn paths_under(&self, prefix: &str) -> Vec<String> {
        let prefix = lookup_key(prefix, self.policy);
        let mut paths: Vec<String> = self
            .lookup
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(_, &(pak, entry))| self.paks[pak].entries[entry].path.clone())
            .collect();
        paths.sort();
        paths
    }
}

impl AssetSource for PakSet {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let (pak, entry) = self.entry(path).ok_or_else(|| Error::NotInAnyPak {
            path: path.to_string(),
        })?;
        pak.read_with_policy(entry, self.policy)
    }
    fn exists(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }
}
