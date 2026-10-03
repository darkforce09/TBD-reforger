//! Loose-file resolution and ordered asset-source fallback.
//!
//! **Role:** serves hand-extracted folders ([`DirSource`]) and an ordered list of sources
//! ([`LayeredSource`]) through [`crate::AssetSource`].
//! **Position:** beside [`crate::PakSet`]; the blueprint compiler layers paks and loose folders
//! through these types.
//! **Signals & state:** none; reads the filesystem on each call.
//! **Invariants:** an exact path wins over a case-insensitive match; a layered read comes from
//! the first source that holds the path, so a read error there is never masked by a later source.

use std::{fs, path::PathBuf};

use crate::{AssetSource, Error, Result};

/// A loose directory tree (the operator's hand-extracted files, or a test fixture dir).
/// Path lookup is exact first, then case-insensitive within the same directory.
#[derive(Debug)]
pub struct DirSource {
    /// The folder virtual paths are resolved under.
    pub root: PathBuf,
}

impl DirSource {
    fn resolve(&self, rel_path: &str) -> Option<PathBuf> {
        let rel = rel_path.replace('\\', "/");
        let direct = self.root.join(&rel);
        if direct.is_file() {
            return Some(direct);
        }
        // Case-insensitive walk, one component at a time.
        let mut cur = self.root.clone();
        for comp in rel.trim_start_matches('/').split('/') {
            let want = comp.to_ascii_lowercase();
            let next = fs::read_dir(&cur)
                .ok()?
                .filter_map(|e| e.ok())
                .find(|e| e.file_name().to_string_lossy().to_ascii_lowercase() == want)?;
            cur = next.path();
        }
        cur.is_file().then_some(cur)
    }
}

impl AssetSource for DirSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>> {
        let p = self.resolve(rel_path).ok_or_else(|| Error::NotUnderRoot {
            path: rel_path.to_string(),
            root: self.root.clone(),
        })?;
        fs::read(&p).map_err(|cause| Error::File { path: p, cause })
    }
    fn exists(&self, rel_path: &str) -> bool {
        self.resolve(rel_path).is_some()
    }
}

/// Paks first, loose files as a fallback (and vice versa when only a directory exists).
pub struct LayeredSource {
    /// The sources, asked in order.
    pub layers: Vec<Box<dyn AssetSource>>,
}

impl AssetSource for LayeredSource {
    fn read(&self, rel_path: &str) -> Result<Vec<u8>> {
        for l in &self.layers {
            if l.exists(rel_path) {
                return l.read(rel_path);
            }
        }
        Err(Error::NotInAnySource {
            path: rel_path.to_string(),
        })
    }
    fn exists(&self, rel_path: &str) -> bool {
        self.layers.iter().any(|l| l.exists(rel_path))
    }
}
