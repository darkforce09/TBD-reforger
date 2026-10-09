//! The source trees of the frontend packages, one per package.
//!
//! **Role:** answers "which `src/` folders make up the frontend" for every test that scans the
//! whole frontend (the single-page app's documentation audit, the Mission Creator's
//! whole-frontend source pins), and checks that a scan reads each package once.
//! **Position:** test-only support over the checkout root of [`crate::repository_root`]; walks
//! the folders the workspace member glob [`FRONTEND_CRATE_MEMBER_GLOB`] reaches, the shell
//! layer's app and service worker included.
//! **Signals & state:** none; reads the file system.
//! **Invariants:** every frontend package appears exactly once, keyed by the package name its
//! manifest declares; a list that names one package twice panics instead of reading its files
//! twice. The walk fails closed: a missing member glob, a crate folder without its manifest, its
//! package name or its `src/`, or an unreadable folder panics instead of shrinking the walk.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The workspace member glob that reaches every frontend crate, as the root `Cargo.toml` spells
/// it: one folder per layer, one crate folder per layer entry.
pub const FRONTEND_CRATE_MEMBER_GLOB: &str = "crates/frontend/*/*";

/// The folder the member glob expands under, relative to the repository root.
const FRONTEND_CRATES_FOLDER: &str = "crates/frontend";

/// The `src/` folder of every frontend package under `repository`, one per package, sorted by
/// path.
///
/// # Panics
///
/// When the root `Cargo.toml` declares no [`FRONTEND_CRATE_MEMBER_GLOB`] member, when the glob
/// reaches no crate, when a folder it reaches holds no `Cargo.toml`, no package name or no
/// `src/`, or when a folder cannot be read.
pub fn frontend_source_roots(repository: &Path) -> Vec<PathBuf> {
    let workspace_manifest = fs::read_to_string(repository.join("Cargo.toml"))
        .expect("read the workspace's root Cargo.toml");
    let member_line = format!("\"{FRONTEND_CRATE_MEMBER_GLOB}\",");
    assert!(
        workspace_manifest
            .lines()
            .any(|line| line.trim() == member_line),
        "the root Cargo.toml declares no `{FRONTEND_CRATE_MEMBER_GLOB}` member: the frontend \
         crates are found through that glob"
    );
    let mut roots = Vec::new();
    for layer in subfolders(&repository.join(FRONTEND_CRATES_FOLDER)) {
        for crate_folder in subfolders(&layer) {
            assert!(
                crate_folder.join("Cargo.toml").is_file(),
                "{} matches `{FRONTEND_CRATE_MEMBER_GLOB}` but holds no Cargo.toml",
                crate_folder.display()
            );
            let source = crate_folder.join("src");
            assert!(
                source.is_dir(),
                "frontend crate {} has no src/ folder",
                crate_folder.display()
            );
            roots.push(source);
        }
    }
    assert!(
        !roots.is_empty(),
        "`{FRONTEND_CRATE_MEMBER_GLOB}` reaches no crate under {}",
        repository.join(FRONTEND_CRATES_FOLDER).display()
    );
    roots.sort();
    assert_each_package_read_once(&roots);
    roots
}

/// Checks that `source_roots`, a list of `src/` folders a scan reads, names each package once.
///
/// A root's package is the `name` of the `[package]` table in the `Cargo.toml` beside it, so two
/// spellings of one folder, or two folders of one package, count as the same package.
///
/// # Panics
///
/// When two roots belong to one package (the message names the package and both roots), or when
/// a root has no manifest beside it or its manifest declares no package name.
pub fn assert_each_package_read_once(source_roots: &[PathBuf]) {
    let mut seen: BTreeMap<String, &Path> = BTreeMap::new();
    for root in source_roots {
        let package = package_of_source_root(root);
        if let Some(first) = seen.insert(package.clone(), root) {
            panic!(
                "the scan reads package `{package}` twice: {} and {}",
                first.display(),
                root.display()
            );
        }
    }
}

/// The package name declared by the manifest beside the `src/` folder `root`.
///
/// # Panics
///
/// When `root` has no parent folder, the manifest cannot be read, or its `[package]` table holds
/// no `name = "…"` line.
fn package_of_source_root(root: &Path) -> String {
    let manifest = root
        .parent()
        .unwrap_or_else(|| panic!("source root {} has no parent folder", root.display()))
        .join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest.display()));
    let mut in_package_table = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package_table = line == "[package]";
            continue;
        }
        if !in_package_table {
            continue;
        }
        let Some(value) = line
            .strip_prefix("name")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='))
        else {
            continue;
        };
        if let Some(name) = value
            .trim()
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
        {
            return name.to_owned();
        }
    }
    panic!(
        "{} declares no package name in its [package] table",
        manifest.display()
    )
}

/// The folders directly inside `dir`, sorted by path.
///
/// # Panics
///
/// When `dir` or one of its entries cannot be read.
fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read folder {}: {error}", dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("read an entry of {}: {error}", dir.display()))
                .path()
        })
        .filter(|path| path.is_dir())
        .collect();
    out.sort();
    out
}

#[cfg(test)]
#[path = "tests/frontend_source_roots.rs"]
mod tests;
