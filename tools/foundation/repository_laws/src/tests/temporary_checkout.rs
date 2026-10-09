//! A throwaway checkout for the repository-law tests.
//!
//! **Role:** builds an empty repository root in the system temporary directory, so a test plants
//! exactly the files it is about, and finds the checkout this crate is built from.
//! **Position:** test support for the sibling test modules of [`super`].
//! **Signals & state:** each [`TemporaryCheckout`] owns one directory and removes it on drop.
//! **Invariants:** the directory name carries the process id and the test's own name, so tests
//! running in parallel never share a checkout.

use std::path::{Path, PathBuf};

/// A temporary repository root.
pub(super) struct TemporaryCheckout(PathBuf);

impl TemporaryCheckout {
    /// A checkout holding nothing at all.
    pub(super) fn empty(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("repository-laws-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    /// The repository root.
    pub(super) fn root(&self) -> &Path {
        &self.0
    }

    /// Write `body` to the repository-relative `rel`, creating its folders.
    pub(super) fn write(&self, rel: &str, body: &str) {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
}

impl Drop for TemporaryCheckout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The repository this crate is built from: the checkout root `repository_root` finds above the
/// crate's manifest folder.
pub(super) fn this_repository() -> PathBuf {
    repository_root::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the crate sits inside a checkout holding the root marker")
}
