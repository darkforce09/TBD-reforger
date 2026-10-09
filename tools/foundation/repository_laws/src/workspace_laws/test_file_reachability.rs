//! The test-file reachability law: every Rust file in a test folder is compiled by some target.
//!
//! **Role:** finds every `.rs` file of a workspace member that sits in a `tests` folder — under
//! `src/` (sibling unit tests) or the member's `tests/` folder (integration test support) — and
//! that no module declaration of the member reaches. Such a file is invisible to the compiler: its
//! tests never run while the tree looks covered.
//! **Position:** `cargo xtask verify test-file-reachability` prints
//! [`check_test_file_reachability`]; it sits beside crate anatomy among the workspace laws
//! because it judges each member as a crate (its targets, its module tree).
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a member's module tree is walked from the root files of its targets — the
//! manifest's `[lib]` and `[[bin]]` paths, `src/lib.rs`, `src/main.rs`, `src/bin/*.rs`,
//! `src/bin/*/main.rs`, and the integration targets `tests/*.rs`, `tests/*/main.rs`,
//! `benches/*.rs` and `examples/*.rs` — through every `mod` declaration with its `#[path]`
//! (`module_declarations`), and through every trybuild case a reached file names
//! (`.compile_fail("…")` or `.pass("…")`, member-relative, `*` and `?` matching within one path
//! segment), which trybuild compiles as a crate of its own; a file is reachable when that walk
//! loads it. An integration target is never itself a finding. The walk skips `target` folders
//! and any folder holding its own `Cargo.toml` (another package). A missing member folder or an
//! unreadable source is [`NotRun`].

mod module_declarations;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{LawOutcome, WorkspaceLawReport};
use crate::workspace_members::{
    WorkspaceMember, child_folder_names, read_workspace_members, wildcard_matches,
};
use module_declarations::{ModuleFile, declared_module_files, normalised};
use regex::Regex;
use verification_core::verdict::NotRun;

/// The folder name that marks a test file: a sibling unit-test folder under `src/` or the
/// member's integration-test folder.
pub const TEST_FOLDER: &str = "tests";

/// A test file no target of its member compiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreachableTestFile {
    /// The file, repository-relative with `/` separators.
    pub path: String,
    /// The package of the member it sits in.
    pub package: String,
}

impl UnreachableTestFile {
    /// The finding line the law prints.
    pub fn rendered(&self) -> String {
        format!(
            "{}: no module declaration of `{}` reaches this test file — declare it from the file \
             it tests (`#[cfg(test)] #[path = \"tests/<file>.rs\"] mod tests;`) or from an \
             integration target, or delete it",
            self.path, self.package
        )
    }
}

/// The test-file reachability report over the checkout at `repo_root`.
pub fn check_test_file_reachability(repo_root: &Path) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "test-file-reachability",
        test_file_reachability_outcome(repo_root),
    )
}

/// The findings of the test-file reachability law; [`NotRun`] when an input could not be read.
pub fn test_file_reachability_outcome(repo_root: &Path) -> Result<LawOutcome, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let mut test_files = 0;
    let mut findings = Vec::new();
    for member in &members {
        let scan = member_scan(repo_root, member)?;
        test_files += scan.test_files;
        findings.extend(scan.unreachable.iter().map(UnreachableTestFile::rendered));
    }
    Ok(LawOutcome {
        summary: format!(
            "{} workspace member(s), {test_files} file(s) in test folders",
            members.len()
        ),
        findings,
        notes: Vec::new(),
    })
}

/// Every unreachable test file of every member of the checkout at `repo_root`.
pub fn unreachable_test_files(repo_root: &Path) -> Result<Vec<UnreachableTestFile>, NotRun> {
    let mut found = Vec::new();
    for member in &read_workspace_members(repo_root)? {
        found.extend(member_scan(repo_root, member)?.unreachable);
    }
    Ok(found)
}

/// What the law found in one member.
struct MemberScan {
    /// How many `.rs` files sit in its test folders, integration targets excluded.
    test_files: usize,
    /// The ones no target reaches.
    unreachable: Vec<UnreachableTestFile>,
}

/// The test files of `member` and the ones its module tree does not reach.
fn member_scan(repo_root: &Path, member: &WorkspaceMember) -> Result<MemberScan, NotRun> {
    let member_root = repo_root.join(&member.path);
    if !member_root.is_dir() {
        return Err(NotRun::TargetMissing(member_root));
    }
    let mut sources = Vec::new();
    collect_sources(&member_root, "", &mut sources)?;
    let roots = target_roots(member, &sources);
    let reached = reachable_files(&member_root, &roots, &sources)?;
    let in_test_folders: Vec<&String> = sources
        .iter()
        .filter(|rel| sits_in_test_folder(rel) && !roots.contains(*rel))
        .collect();
    let unreachable = in_test_folders
        .iter()
        .filter(|rel| !reached.contains(Path::new(rel.as_str())))
        .map(|rel| UnreachableTestFile {
            path: format!("{}/{rel}", member.path),
            package: member.package_name.clone(),
        })
        .collect();
    Ok(MemberScan {
        test_files: in_test_folders.len(),
        unreachable,
    })
}

/// True for a member-relative file under `src/` with a [`TEST_FOLDER`] folder on its path, or
/// under the member's own [`TEST_FOLDER`].
fn sits_in_test_folder(rel: &str) -> bool {
    let folders: Vec<&str> = rel.split('/').collect();
    let folders = &folders[..folders.len().saturating_sub(1)];
    match folders.first() {
        Some(&"src") => folders.contains(&TEST_FOLDER),
        Some(&first) => first == TEST_FOLDER,
        None => false,
    }
}

/// The member-relative root files of every target of `member`, among `sources`.
fn target_roots(member: &WorkspaceMember, sources: &[String]) -> BTreeSet<String> {
    let mut roots: BTreeSet<String> = sources
        .iter()
        .filter(|rel| is_conventional_target_root(rel))
        .cloned()
        .collect();
    let declared = member
        .manifest
        .library
        .iter()
        .chain(&member.manifest.binaries)
        .filter_map(|target| target.path.as_deref());
    for path in declared {
        let path = normalised(Path::new(path));
        roots.insert(path.to_string_lossy().replace('\\', "/"));
    }
    roots
}

/// True for the root files Cargo discovers without a manifest entry.
fn is_conventional_target_root(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    matches!(
        parts.as_slice(),
        ["src", "lib.rs" | "main.rs"]
            | ["src", "bin", _]
            | ["tests" | "benches" | "examples", _]
            | ["src", "bin", _, "main.rs"]
            | ["tests", _, "main.rs"]
    )
}

/// Every file the module trees rooted at `roots` load, and the trybuild cases a loaded file
/// names among the member's `sources`, member-relative and normalised.
fn reachable_files(
    member_root: &Path,
    roots: &BTreeSet<String>,
    sources: &[String],
) -> Result<BTreeSet<PathBuf>, NotRun> {
    let trybuild_case = Regex::new(r#"\.(?:compile_fail|pass)\(\s*"([^"]+)"\s*\)"#)
        .expect("the trybuild case pattern compiles");
    let mut reached: BTreeSet<PathBuf> = BTreeSet::new();
    let mut pending: Vec<ModuleFile> = roots
        .iter()
        .map(|rel| ModuleFile {
            path: PathBuf::from(rel),
            owns_folder: true,
        })
        .collect();
    while let Some(file) = pending.pop() {
        let absolute = member_root.join(&file.path);
        if !absolute.is_file() || !reached.insert(file.path.clone()) {
            continue;
        }
        let text = std::fs::read_to_string(&absolute).map_err(|source| NotRun::Unreadable {
            path: absolute.clone(),
            source,
        })?;
        pending.extend(declared_module_files(&file.path, file.owns_folder, &text));
        for case in trybuild_case.captures_iter(&text) {
            let pattern: Vec<&str> = case[1].split('/').collect();
            pending.extend(
                sources
                    .iter()
                    .filter(|rel| matches_case(&pattern, rel))
                    .map(|rel| ModuleFile {
                        path: PathBuf::from(rel),
                        owns_folder: true,
                    }),
            );
        }
    }
    Ok(reached)
}

/// True when the member-relative `rel` matches the trybuild case `pattern`, segment by segment.
fn matches_case(pattern: &[&str], rel: &str) -> bool {
    let segments: Vec<&str> = rel.split('/').collect();
    segments.len() == pattern.len()
        && pattern
            .iter()
            .zip(&segments)
            .all(|(pattern, segment)| wildcard_matches(pattern, segment))
}

/// Every `.rs` file under `folder` (member-relative prefix `rel`), outside `target` folders and
/// folders holding their own `Cargo.toml`.
fn collect_sources(folder: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), NotRun> {
    let entries = std::fs::read_dir(folder).map_err(|source| NotRun::Unreadable {
        path: folder.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| NotRun::Unreadable {
            path: folder.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if path.is_file() && name.ends_with(".rs") {
            out.push(child_rel);
        }
    }
    for name in child_folder_names(folder)? {
        let child = folder.join(&name);
        if name == "target" || child.join("Cargo.toml").is_file() {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        collect_sources(&child, &child_rel, out)?;
    }
    out.sort();
    Ok(())
}
