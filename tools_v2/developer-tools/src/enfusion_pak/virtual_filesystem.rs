//! Sorted archive merging and policy-specific virtual path resolution.
use super::{PakEntry, PakIndex, ReadPolicy};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A virtual game-file source, including archives and loose extracted directories.
pub trait AssetSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>>;
    fn exists(&self, rel_path: &str) -> bool;
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

#[derive(Debug)]
pub struct PakSet {
    pub(super) paks: Vec<PakIndex>,
    lookup: HashMap<String, (usize, usize)>,
    policy: ReadPolicy,
}

impl PakSet {
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/enfusion-mcp-root/addons"))
    }

    pub fn from_dir(dir: &Path) -> Result<Self> {
        Self::from_dir_with_policy(dir, ReadPolicy::Blueprint)
    }

    pub(crate) fn from_dir_with_policy(dir: &Path, policy: ReadPolicy) -> Result<Self> {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .with_context(|| dir.display().to_string())?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pak"))
            })
            .collect();
        paths.sort();
        if paths.is_empty() {
            bail!("no .pak files under {}", dir.display());
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
                    eprintln!("pak: failed to parse {}: {error}", path.display());
                }
                Err(error) => return Err(error),
            }
        }
        Ok(set)
    }

    /// Later archives never shadow an existing virtual path.
    pub fn push(&mut self, index: PakIndex) {
        let pak = self.paks.len();
        for (entry, item) in index.entries.iter().enumerate() {
            self.lookup
                .entry(lookup_key(&item.path, self.policy))
                .or_insert((pak, entry));
        }
        self.paks.push(index);
    }

    pub fn pak_count(&self) -> usize {
        self.paks.len()
    }
    pub fn file_count(&self) -> usize {
        self.lookup.len()
    }
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
        let (pak, entry) = self
            .entry(path)
            .with_context(|| format!("{path}: not in any pak"))?;
        pak.read_with_policy(entry, self.policy)
    }
    fn exists(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }
}
