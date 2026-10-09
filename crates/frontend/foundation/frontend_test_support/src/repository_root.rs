//! Repository files for tests, found from the calling crate's manifest folder.
//!
//! **Role:** finds the repository root and reads the repository files a test needs from outside
//! its own crate (the captured API responses, the contract schemas, the terrain manifests, the
//! API route tables), so a test reads the same file at whatever depth its crate sits.
//! **Position:** test-only support over the workspace's one root finder
//! ([`::repository_root::find_repository_root_from`]). `golden!`, the fixture helpers and the guard
//! tests call it with `env!("CARGO_MANIFEST_DIR")` expanded in their own crate, because `env!`
//! expands in the crate that spells it.
//! **Signals & state:** one process-wide cache of file texts; each file is read once and its text
//! lives for the rest of the test process, which is what lets a reader hand out `&'static str`.
//! **Invariants:** the root is the nearest ancestor of the manifest folder that holds the
//! [`::repository_root::ROOT_MARKER`] file; a missing root or an unreadable file panics with the
//! path it looked for, so a moved file fails its test instead of passing on empty text.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

/// The repository root: the nearest folder at or above `manifest_dir` that holds the
/// [`::repository_root::ROOT_MARKER`] file.
///
/// `manifest_dir` is the caller's `env!("CARGO_MANIFEST_DIR")`.
///
/// # Panics
///
/// When no ancestor of `manifest_dir` holds the marker; the message names the folder searched
/// from and the marker.
pub fn repository_root(manifest_dir: &str) -> PathBuf {
    ::repository_root::find_repository_root_from(Path::new(manifest_dir))
        .unwrap_or_else(|error| panic!("no repository root at or above {manifest_dir}: {error}"))
}

/// The absolute path of `repository_path`, a `/`-separated path relative to the repository root
/// (for example `contracts/definitions/vehicle-database.schema.json`).
pub fn repository_path(manifest_dir: &str, repository_path: &str) -> PathBuf {
    repository_root(manifest_dir).join(repository_path)
}

/// The text of the repository file at `repository_path`, read once per test process.
///
/// # Panics
///
/// When the repository root cannot be found or the file cannot be read as UTF-8 text.
pub fn repository_text(manifest_dir: &str, repository_path: &str) -> &'static str {
    cached_text(&self::repository_path(manifest_dir, repository_path))
}

/// The text of the file at `path`, read on first use and kept for the process.
///
/// # Panics
///
/// When the file cannot be read as UTF-8 text.
pub fn cached_text(path: &Path) -> &'static str {
    static TEXTS: OnceLock<Mutex<HashMap<PathBuf, &'static str>>> = OnceLock::new();
    let mut texts = TEXTS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(text) = texts.get(path) {
        return text;
    }
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let text: &'static str = Box::leak(text.into_boxed_str());
    texts.insert(path.to_path_buf(), text);
    text
}
