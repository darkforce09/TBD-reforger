//! The cache folders `cargo xtask fetch` fills inside the vanilla reference lane.
//!
//! **Role:** Resolves one cache folder inside the vanilla lane and creates it, only when the
//! references folder already exists.
//!
//! **Position:** Called first by [`crate::vanilla_page_fetch::vanilla_api`] and
//! [`crate::vanilla_page_fetch::vanilla_source`]; the references folder and the cache folders come
//! from [`repository_layout`].
//!
//! **Signals & state:** none; creates folders inside the references folder.
//!
//! **Invariants:** the references folder is never created here (its README.md is tracked, so a
//! checkout without it is not this repository), and every created folder lies inside the vanilla
//! lane.

use std::fs;
use std::path::{Path, PathBuf};

use repository_layout::REFERENCES_DIR;

use crate::{Error, Result};

/// Create and return `<repo_root>/<lane_folder>`, one of the vanilla lane's cache folders
/// ([`repository_layout::VANILLA_SCRIPT_API_PAGES`], [`repository_layout::VANILLA_SOURCE_PAGES`]),
/// refusing when the references folder is absent.
pub(crate) fn prepare_vanilla_cache(repo_root: &Path, lane_folder: &str) -> Result<PathBuf> {
    let references = repo_root.join(REFERENCES_DIR);
    if !references.is_dir() {
        return Err(Error::VanillaLaneMissing { references });
    }
    let cache = repo_root.join(lane_folder);
    fs::create_dir_all(&cache).map_err(|cause| Error::CreateCache {
        cache: cache.clone(),
        cause,
    })?;
    Ok(cache)
}
