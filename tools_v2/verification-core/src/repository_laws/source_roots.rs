//! The source trees the structural laws read, and the one rule that tells a test file apart.
//!
//! **Role:** names the directories the file-length, sibling-test and exemption rules walk — every
//! workspace member folder the root manifest names, plus the pinned Enfusion script roots —
//! walks them fail-closed, and classifies a repository-relative path as a test file or a
//! production file.
//! **Position:** the shared input of [`super::file_length`], [`super::sibling_test_placement`]
//! and [`super::exemption_mechanisms`]; built on [`super::workspace_members`] and
//! [`crate::scan::walk_files`].
//! **Signals & state:** none; constants and pure functions over the checkout.
//! **Invariants:** the Rust roots are the workspace members and nothing else, so a crate is judged
//! from the commit that makes it a member. A missing root manifest, a workspace that names no
//! member, an explicit member folder that is missing and a missing script root are each
//! [`NotRun::TargetMissing`], never a smaller walk. A member nested inside another member is
//! walked once, as part of the outer one. Only the shipped addon script roots may be pinned, which
//! a compile-time assertion holds.

use std::path::{Path, PathBuf};

use super::workspace_members::read_workspace_members;
use crate::scan;
use crate::verdict::NotRun;

/// Enfusion script roots every structural law walks beside the workspace members. A missing root
/// is [`NotRun::TargetMissing`], never an empty pass.
pub const PINNED_SCRIPT_ROOTS: &[&str] = &[
    "apps/mod/tbd-framework/Scripts",
    "apps/mod/tbd-emcp/Scripts",
];

/// Enfusion script roots of the three shipped addons, the only `apps/mod` trees the laws may pin.
/// `apps/mod/crf_framework` and `apps/mod/vanilla_reference` are gitignored upstream references
/// and never enter [`PINNED_SCRIPT_ROOTS`]. Each root joins the pins once its addon's scripts sit
/// at or under the ceilings.
pub const MOD_SCRIPT_ROOTS: &[&str] = &[
    "apps/mod/tbd-framework/Scripts",
    "apps/mod/tbd-export/Scripts",
    "apps/mod/tbd-emcp/Scripts",
];

/// File extensions the length law counts: Rust sources and Enfusion scripts.
pub const LENGTH_GATED_EXTENSIONS: &[&str] = &["rs", "c"];

const _: () = assert!(
    mod_pins_are_script_roots(PINNED_SCRIPT_ROOTS, MOD_SCRIPT_ROOTS),
    "an apps/mod pin in PINNED_SCRIPT_ROOTS must be one of MOD_SCRIPT_ROOTS"
);

/// Compile-time guard: every `apps/mod/` entry of `pins` is exactly one of `script_roots`, so a
/// gitignored reference tree or a whole addon folder can never be pinned.
pub const fn mod_pins_are_script_roots(pins: &[&str], script_roots: &[&str]) -> bool {
    let mut pin_index = 0;
    while pin_index < pins.len() {
        let pin = pins[pin_index].as_bytes();
        if bytes_start_with(pin, b"apps/mod/") || bytes_equal(pin, b"apps/mod") {
            let mut root_index = 0;
            let mut matched = false;
            while root_index < script_roots.len() {
                if bytes_equal(pin, script_roots[root_index].as_bytes()) {
                    matched = true;
                }
                root_index += 1;
            }
            if !matched {
                return false;
            }
        }
        pin_index += 1;
    }
    true
}

const fn bytes_start_with(bytes: &[u8], prefix: &[u8]) -> bool {
    if bytes.len() < prefix.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if bytes[index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn bytes_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bytes_start_with(left, right)
}

/// The absolute roots the structural laws walk: the folder of every workspace member that no other
/// member folder contains, sorted by path, then [`PINNED_SCRIPT_ROOTS`].
pub fn law_source_roots(repo_root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let members = read_workspace_members(repo_root)?;
    if members.is_empty() {
        return Err(NotRun::TargetMissing(repo_root.join("Cargo.toml")));
    }
    let member_folders: Vec<&str> = members.iter().map(|member| member.path.as_str()).collect();
    let mut roots: Vec<PathBuf> = outermost_folders(&member_folders)
        .into_iter()
        .map(|folder| repo_root.join(folder))
        .collect();
    roots.extend(PINNED_SCRIPT_ROOTS.iter().map(|root| repo_root.join(root)));
    Ok(roots)
}

/// The repository-relative `folders` that no other entry of `folders` contains, in input order.
pub fn outermost_folders<'a>(folders: &[&'a str]) -> Vec<&'a str> {
    folders
        .iter()
        .copied()
        .filter(|folder| {
            !folders
                .iter()
                .any(|outer| folder.len() > outer.len() && folder.starts_with(&format!("{outer}/")))
        })
        .collect()
}

/// Every file under [`law_source_roots`] that `keep` accepts, sorted, each once.
pub fn walk_law_sources(
    repo_root: &Path,
    keep: impl Fn(&Path) -> bool,
) -> Result<Vec<PathBuf>, NotRun> {
    let roots = law_source_roots(repo_root)?;
    let refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    let mut files = scan::walk_files(&refs, keep)?;
    files.dedup();
    Ok(files)
}

/// Every file under the law roots whose extension is in [`LENGTH_GATED_EXTENSIONS`].
pub fn walk_length_gated_sources(repo_root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    walk_law_sources(repo_root, |path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| LENGTH_GATED_EXTENSIONS.contains(&ext))
    })
}

/// Every Rust source under the law roots.
pub fn walk_rust_sources(repo_root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    walk_law_sources(repo_root, |path| {
        path.extension().is_some_and(|ext| ext == "rs")
    })
}

/// `file` relative to `repo_root` with `/` separators, as a finding names it.
pub fn repository_relative(repo_root: &Path, file: &Path) -> String {
    file.strip_prefix(repo_root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// True when the repository-relative `rel` is a test file: a path component is `tests`, or a
/// `.rs` or `.c` stem ends in `_tests`.
pub fn is_test_file(rel: &str) -> bool {
    let path = Path::new(rel);
    path.components().any(|part| part.as_os_str() == "tests")
        || (path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| LENGTH_GATED_EXTENSIONS.contains(&extension))
            && path
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy().ends_with("_tests")))
}

#[cfg(test)]
#[path = "tests/source_roots.rs"]
mod tests;
