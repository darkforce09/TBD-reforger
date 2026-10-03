//! The cache folders `cargo xtask fetch` fills inside the vanilla reference lane.
//!
//! **Role:** Resolves one cache folder inside the vanilla lane and creates it, only when the
//! references folder already exists.
//!
//! **Position:** Called first by [`crate::commands::fetch::vanilla_api`] and
//! [`crate::commands::fetch::vanilla_source`]; the lane locations come from
//! [`crate::core::repository_layout`].
//!
//! **Signals & state:** none; creates folders inside the references folder.
//!
//! **Invariants:** the references folder is never created here (its README.md is tracked, so a
//! checkout without it is not this repository), and every created folder lies inside the vanilla
//! lane.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use repository_layout::{REFERENCES_DIR, VANILLA_REFERENCE};

/// The Script API pages `cargo xtask fetch vanilla-api` caches.
pub(crate) const SCRIPT_API_PAGES: &str = "apidoc";

/// The source pages `cargo xtask fetch vanilla-source` caches.
pub(crate) const SOURCE_PAGES: &str = "source_html";

/// Create and return `<repo_root>/apps/mod/References/vanilla_reference/<folder>`, refusing when
/// the references folder is absent.
pub(crate) fn prepare_vanilla_cache(repo_root: &Path, folder: &str) -> Result<PathBuf> {
    let references = repo_root.join(REFERENCES_DIR);
    if !references.is_dir() {
        bail!(
            "{} is missing; the vanilla lane is written only inside it (its README.md is \
             tracked, so run from a checkout of this repository)",
            references.display()
        );
    }
    let cache = repo_root.join(VANILLA_REFERENCE).join(folder);
    fs::create_dir_all(&cache).with_context(|| format!("mkdir -p {}", cache.display()))?;
    Ok(cache)
}
