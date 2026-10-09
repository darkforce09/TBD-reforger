//! Isolated Git inventories distinguish source bytes, tracked deletions, tracked symlinks and
//! invalid inputs.
use super::*;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Repository {
    root: PathBuf,
}

impl Repository {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let repository = Self {
            root: std::env::temp_dir().join(format!(
                "tbd-source-fingerprint-{}-{unique}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
        };
        fs::create_dir(&repository.root).unwrap();
        repository.git(&["init", "--quiet", "--initial-branch=main"]);
        repository
    }

    fn git(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&self.root)
            .output()
            .expect("run isolated Git fixture command");
        assert!(
            output.status.success(),
            "Git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Git fixture output is UTF-8")
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn seed(&self) {
        self.write("Cargo.toml", "[workspace]\n");
        self.write("mod/module.rs", "pub fn verified() {}\n");
        self.git(&["add", "--", "Cargo.toml", "mod/module.rs"]);
    }

    fn fingerprint(&self) -> String {
        source(&self.root).unwrap()
    }

    #[cfg(unix)]
    fn refusal(&self) -> String {
        source(&self.root)
            .expect_err("fingerprint must refuse this tree")
            .to_string()
    }

    /// Replaces whatever sits at `path` with a symlink holding `target` as its link text.
    #[cfg(unix)]
    fn link(&self, target: impl AsRef<Path>, path: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        if fs::symlink_metadata(&path).is_ok() {
            fs::remove_file(&path).unwrap();
        }
        std::os::unix::fs::symlink(target, path).unwrap();
    }

    #[cfg(unix)]
    fn index_mode(&self, path: &str) -> String {
        let entry = self.git(&["ls-files", "--stage", "--", path]);
        entry.split(' ').next().unwrap_or_default().to_owned()
    }

    /// Seeds the workspace plus `AGENTS.md`, tracked with Git mode 120000 as a link to
    /// `CLAUDE.md`, the layout of the real repository root.
    #[cfg(unix)]
    fn seed_with_tracked_symlink(&self) {
        self.seed();
        self.write("CLAUDE.md", "# Instructions\n");
        self.link("CLAUDE.md", "AGENTS.md");
        self.git(&["add", "--", "CLAUDE.md", "AGENTS.md"]);
        assert_eq!(self.index_mode("AGENTS.md"), "120000");
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn content_empty_file_and_tracked_worktree_deletion_have_distinct_fingerprints() {
    let repository = Repository::new();
    repository.seed();
    let content = repository.fingerprint();
    repository.write("mod/module.rs", "");
    let empty = repository.fingerprint();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    let deleted = repository.fingerprint();
    assert_eq!(deleted, repository.fingerprint());
    assert_eq!(BTreeSet::from([content, empty, deleted]).len(), 3);
}

#[test]
fn staged_deletion_removes_the_indexed_tombstone_deterministically() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    let worktree_deletion = repository.fingerprint();
    repository.git(&["add", "--update", "--", "mod/module.rs"]);
    let staged_deletion = repository.fingerprint();
    assert_eq!(staged_deletion, repository.fingerprint());
    assert_eq!(
        BTreeSet::from([original, worktree_deletion, staged_deletion.clone()]).len(),
        3
    );
    repository.write("mod/module.rs", "pub fn verified() {}\n");
    assert_ne!(repository.fingerprint(), staged_deletion);
}

#[test]
fn staged_rename_binds_the_new_path_and_remains_deterministic() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    fs::rename(
        repository.root.join("mod/module.rs"),
        repository.root.join("mod/renamed.rs"),
    )
    .unwrap();
    let unstaged = repository.fingerprint();
    assert_ne!(unstaged, original);
    repository.git(&["add", "--all", "--", "mod"]);
    let staged = repository.fingerprint();
    assert_ne!(staged, original);
    assert_ne!(staged, unstaged);
    assert_eq!(staged, repository.fingerprint());
    repository.git(&["mv", "--", "mod/renamed.rs", "mod/module.rs"]);
    assert_eq!(repository.fingerprint(), original);
}

#[test]
fn additions_and_restoration_change_the_hash_and_staging_present_files_does_not() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    repository.write("mod/addition.rs", "pub const NEW: bool = true;\n");
    let added = repository.fingerprint();
    assert_ne!(added, original);
    repository.git(&["add", "--", "mod/addition.rs"]);
    assert_eq!(repository.fingerprint(), added);
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    assert_ne!(repository.fingerprint(), added);
    repository.write("mod/module.rs", "pub fn verified() {}\n");
    assert_eq!(repository.fingerprint(), added);
}

#[test]
fn a_change_under_an_api_crate_changes_the_fingerprint() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    let crate_source = "crates/api/api_state/src/lib.rs";
    repository.write(crate_source, "pub struct AppState;\n");
    repository.git(&["add", "--", crate_source]);
    let added = repository.fingerprint();
    assert_ne!(
        added, original,
        "an API crate source is a fingerprint input"
    );
    repository.write(crate_source, "pub struct AppState(u8);\n");
    assert_ne!(repository.fingerprint(), added, "its content is hashed");
    assert!(source_input(crate_source));
    assert!(source_input("crates/api/api_state/Cargo.toml"));
}

/// A path is an input only inside one of the input folders: every folder the repository layout
/// names counts, and a sibling whose name merely starts with an input folder's name does not.
#[test]
fn only_paths_inside_an_input_folder_are_source_inputs() {
    for inside in [
        "mod/tbd-framework/addon.gproj",
        "crates/api/api_server/src/lib.rs",
        "crates/geometry/geometry_primitives/src/lib.rs",
        "tools/xtask/src/main.rs",
        "contracts/definitions/mission.schema.json",
        "documentation/crates/api/api_server/verification_evidence/register.md",
        ".cargo/config.toml",
        ".github/workflows/ci.yml",
    ] {
        assert!(source_input(inside), "{inside} is inside an input folder");
    }
    for outside in [
        "modx/main.rs",
        "mod.rs",
        "crates.rs",
        "toolset/lib.rs",
        "contracts_old/schema.json",
        "documentation/crates/api/api_server/README.md",
        ".github-old/ci.yml",
    ] {
        assert!(
            !source_input(outside),
            "{outside} lies outside every input folder"
        );
    }
}

#[test]
fn disappearance_or_restoration_after_inventory_is_rejected() {
    let repository = Repository::new();
    repository.seed();
    let inventory = source_inventory(&repository.root).unwrap();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    let error = hash_source_inventory(&repository.root, &inventory).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("presence changed after inventory")
    );
    let deleted = source_inventory(&repository.root).unwrap();
    assert!(deleted.deleted.contains("mod/module.rs"));
    repository.write("mod/module.rs", "restored");
    let error = hash_source_inventory(&repository.root, &deleted).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("presence changed after inventory")
    );

    repository.write("mod/untracked.rs", "untracked");
    let untracked = source_inventory(&repository.root).unwrap();
    fs::remove_file(repository.root.join("mod/untracked.rs")).unwrap();
    assert!(hash_source_inventory(&repository.root, &untracked).is_err());
}

#[test]
fn deleted_source_still_fails_required_implementation_register_validation() {
    use super::super::register;
    let repository = Repository::new();
    repository.seed();
    let register: register::Register = serde_json::from_value(serde_json::json!({
        "version": 1,
        "requirements": [{
            "id": "required-source", "behavior": "Required implementation exists",
            "implementation": ["mod/module.rs"], "checks": ["source-check"],
            "assumptions": []
        }],
        "checks": [{
            "id": "source-check", "class": "implementation", "command": null,
            "timeout_seconds": 1, "minimum_cases": 1,
            "success_marker": "verified", "case_pattern": "verified"
        }]
    }))
    .unwrap();
    register::validate(&repository.root, &register).unwrap();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    assert!(
        source(&repository.root).is_ok(),
        "deletion has a fingerprint"
    );
    let error = register::validate(&repository.root, &register).unwrap_err();
    assert!(error.to_string().contains("missing implementation"));
}

#[test]
fn empty_or_non_source_inventory_fails_closed() {
    let repository = Repository::new();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("empty source")
    );
    repository.write("mod/terrain.bin", "binary fixture");
    repository.git(&["add", "--", "mod/terrain.bin"]);
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("empty source")
    );
}

#[test]
fn a_directory_cannot_replace_a_fingerprinted_regular_file() {
    let repository = Repository::new();
    repository.seed();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    fs::create_dir(repository.root.join("mod/module.rs")).unwrap();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("not a regular file")
    );
}

#[cfg(unix)]
#[test]
fn symlink_files_and_ancestor_directories_fail_instead_of_becoming_tombstones() {
    use std::os::unix::fs::symlink;
    let repository = Repository::new();
    repository.seed();
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    symlink("../Cargo.toml", repository.root.join("mod/module.rs")).unwrap();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );
    fs::remove_file(repository.root.join("mod/module.rs")).unwrap();
    symlink("missing-target", repository.root.join("mod/module.rs")).unwrap();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );

    let nested = Repository::new();
    nested.write("mod/source/module.rs", "tracked source");
    nested.git(&["add", "--", "mod/source/module.rs"]);
    fs::rename(
        nested.root.join("mod/source"),
        nested.root.join("relocated"),
    )
    .unwrap();
    symlink("../relocated", nested.root.join("mod/source")).unwrap();
    assert!(
        source(&nested.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );
}

#[cfg(unix)]
#[test]
fn tracked_symlink_is_fingerprinted_stably_by_its_tagged_link_text() {
    let repository = Repository::new();
    repository.seed_with_tracked_symlink();
    let linked = repository.fingerprint();
    assert_eq!(linked, repository.fingerprint());

    repository.write("mod/addition.rs", "pub const NEW: bool = true;\n");
    assert_ne!(repository.fingerprint(), linked, "other inputs stay bound");
    fs::remove_file(repository.root.join("mod/addition.rs")).unwrap();
    assert_eq!(repository.fingerprint(), linked);

    fs::remove_file(repository.root.join("AGENTS.md")).unwrap();
    repository.write("AGENTS.md", "CLAUDE.md");
    repository.git(&["add", "--", "AGENTS.md"]);
    assert_eq!(repository.index_mode("AGENTS.md"), "100644");
    assert_ne!(
        repository.fingerprint(),
        linked,
        "a regular file holding the link text must not collide with the symlink"
    );

    repository.link("CLAUDE.md", "AGENTS.md");
    repository.git(&["add", "--", "AGENTS.md"]);
    assert_eq!(repository.index_mode("AGENTS.md"), "120000");
    assert_eq!(repository.fingerprint(), linked);
}

#[cfg(unix)]
#[test]
fn retargeting_a_tracked_symlink_changes_the_fingerprint() {
    let repository = Repository::new();
    repository.seed_with_tracked_symlink();
    repository.write("INSTRUCTIONS.md", "# Instructions\n");
    repository.git(&["add", "--", "INSTRUCTIONS.md"]);
    let original = repository.fingerprint();

    repository.link("INSTRUCTIONS.md", "AGENTS.md");
    let retargeted = repository.fingerprint();
    assert_ne!(
        retargeted, original,
        "a new link text over identical target bytes is a new fingerprint"
    );
    repository.git(&["add", "--", "AGENTS.md"]);
    assert_eq!(repository.fingerprint(), retargeted);

    repository.link("CLAUDE.md", "AGENTS.md");
    assert_eq!(repository.fingerprint(), original);
}

#[cfg(unix)]
#[test]
fn untracked_symlink_beside_an_accepted_tracked_symlink_is_refused() {
    let repository = Repository::new();
    repository.seed_with_tracked_symlink();
    let accepted = repository.fingerprint();

    repository.link("module.rs", "mod/linked.rs");
    assert!(
        repository
            .refusal()
            .contains("symlink fingerprint input is not tracked as a symlink: mod/linked.rs")
    );
    fs::remove_file(repository.root.join("mod/linked.rs")).unwrap();
    assert_eq!(repository.fingerprint(), accepted);

    repository.link("../Cargo.toml", "mod/module.rs");
    assert_eq!(repository.index_mode("mod/module.rs"), "100644");
    assert!(
        repository
            .refusal()
            .contains("symlink fingerprint input is not tracked as a symlink: mod/module.rs")
    );
}

#[cfg(unix)]
#[test]
fn tracked_symlink_resolving_outside_the_repository_or_nowhere_is_refused() {
    let repository = Repository::new();
    repository.seed_with_tracked_symlink();
    repository.fingerprint();
    let outside = Repository::new();
    outside.write("target.md", "# Instructions\n");
    let outside_name = outside.root.file_name().unwrap().to_str().unwrap();

    for target in [
        outside.root.join("target.md"),
        PathBuf::from(format!("../{outside_name}/target.md")),
        PathBuf::from("hop"),
    ] {
        repository.link(outside.root.join("target.md"), "hop");
        repository.link(&target, "AGENTS.md");
        repository.git(&["add", "--", "AGENTS.md"]);
        assert_eq!(repository.index_mode("AGENTS.md"), "120000");
        assert!(
            repository
                .refusal()
                .contains("symlink fingerprint input resolves outside the repository: AGENTS.md"),
            "{target:?} escapes the repository"
        );
    }

    repository.link("missing.md", "AGENTS.md");
    repository.git(&["add", "--", "AGENTS.md"]);
    assert!(
        repository
            .refusal()
            .contains("symlink fingerprint input does not resolve: AGENTS.md")
    );
}
