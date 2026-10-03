//! A throwaway git checkout on disk for the relocation tests.
//!
//! Files are written, then [`FixtureRepository::track`] runs `git init` and `git add` so the index
//! lists them the way a real checkout's does, and [`FixtureRepository::commit`] records them in
//! the history the manifest chronology reads; the relocation reads the index, the attributes, the
//! history and the disk exactly as it does on the repository. Every git call drops the variables
//! that would point it at another repository or index.

use std::path::{Path, PathBuf};

use super::repository_files::git;

/// A temporary checkout, removed when dropped.
pub(super) struct FixtureRepository {
    root: PathBuf,
}

impl FixtureRepository {
    /// An empty checkout under the system temporary folder, unique per tag, process and moment.
    pub(super) fn new(tag: &str) -> FixtureRepository {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let root = std::env::temp_dir().join(format!(
            "refactor-relocate-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the fixture checkout");
        FixtureRepository { root }
    }

    /// Write `path` with `text`.
    pub(super) fn write(&self, path: &str, text: &str) -> &FixtureRepository {
        self.write_bytes(path, text.as_bytes())
    }

    /// Write `path` with `bytes`.
    pub(super) fn write_bytes(&self, path: &str, bytes: &[u8]) -> &FixtureRepository {
        let full = self.root.join(path);
        if let Some(folder) = full.parent() {
            std::fs::create_dir_all(folder).expect("create a fixture folder");
        }
        std::fs::write(full, bytes).expect("write a fixture file");
        self
    }

    /// Make the checkout a git repository whose index holds every file written so far.
    pub(super) fn track(&self) -> &FixtureRepository {
        git(&self.root, &["init", "--quiet"]).expect("git init");
        git(&self.root, &["add", "--all"]).expect("git add");
        self
    }

    /// Track every file written so far and commit them under a fixed test identity, so the
    /// history orders the manifests the way `main` does.
    pub(super) fn commit(&self, message: &str) -> &FixtureRepository {
        self.track();
        git(
            &self.root,
            &[
                "-c",
                "user.name=relocation tests",
                "-c",
                "user.email=relocation-tests@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--allow-empty",
                "--message",
                message,
            ],
        )
        .expect("git commit");
        self
    }

    /// The text of `path`.
    pub(super) fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"))
    }

    /// The bytes of `path`.
    pub(super) fn read_bytes(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
    }

    /// Whether `path` exists on disk.
    pub(super) fn exists(&self, path: &str) -> bool {
        self.root.join(path).exists()
    }

    /// The paths the index lists, sorted.
    pub(super) fn tracked(&self) -> Vec<String> {
        let listing = git(&self.root, &["ls-files"]).expect("git ls-files");
        listing.lines().map(str::to_string).collect()
    }

    /// Write a manifest outside the checkout and return its path.
    pub(super) fn manifest(&self, rows: &str) -> PathBuf {
        let path = self.root.with_extension("tsv");
        std::fs::write(&path, format!("kind\tfrom\tto\tscope\n{rows}")).expect("write manifest");
        path
    }

    /// The checkout root.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for FixtureRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
        let _ = std::fs::remove_file(self.root.with_extension("tsv"));
    }
}
