//! A throwaway checkout for the repository-law tests.
//!
//! **Role:** builds a repository skeleton in the system temporary directory with every pinned
//! law root present, so a test plants exactly the files it is about and nothing refuses for a
//! missing root.
//! **Position:** test support for the sibling test modules of [`super`].
//! **Signals & state:** each [`TemporaryCheckout`] owns one directory and removes it on drop.
//! **Invariants:** the directory name carries the process id and the test's own name, so tests
//! running in parallel never share a checkout.

use std::path::{Path, PathBuf};

use super::source_roots::FILE_LENGTH_PINS;

/// A temporary repository root.
pub(super) struct TemporaryCheckout(PathBuf);

impl TemporaryCheckout {
    /// A checkout holding every pinned root, each with one one-line `lib.rs`.
    pub(super) fn with_pinned_roots(name: &str) -> Self {
        let checkout = Self::empty(name);
        for pin in FILE_LENGTH_PINS {
            checkout.write(&format!("{pin}/lib.rs"), "fn placeholder() {}\n");
        }
        checkout
    }

    /// A checkout holding nothing at all.
    pub(super) fn empty(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "verification-core-laws-{}-{name}",
            std::process::id()
        ));
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

    /// Write a file of exactly `count` comment lines to `rel`.
    pub(super) fn write_lines(&self, rel: &str, count: usize) {
        self.write(rel, &"// line\n".repeat(count));
    }
}

impl Drop for TemporaryCheckout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The repository this crate is built from: two levels above `tools_v2/verification-core`.
pub(super) fn this_repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("tools_v2/verification-core sits two levels below the repository root")
        .to_path_buf()
}
