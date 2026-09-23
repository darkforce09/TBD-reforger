//! Isolated Git inventories distinguish source bytes, tracked deletions and invalid inputs.
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

    fn git(&self, arguments: &[&str]) {
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
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn seed(&self) {
        self.write("Cargo.toml", "[workspace]\n");
        self.write("apps/module.rs", "pub fn verified() {}\n");
        self.git(&["add", "--", "Cargo.toml", "apps/module.rs"]);
    }

    fn fingerprint(&self) -> String {
        source(&self.root).unwrap()
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
    repository.write("apps/module.rs", "");
    let empty = repository.fingerprint();
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    let deleted = repository.fingerprint();
    assert_eq!(deleted, repository.fingerprint());
    assert_eq!(BTreeSet::from([content, empty, deleted]).len(), 3);
}

#[test]
fn staged_deletion_removes_the_indexed_tombstone_deterministically() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    let worktree_deletion = repository.fingerprint();
    repository.git(&["add", "--update", "--", "apps/module.rs"]);
    let staged_deletion = repository.fingerprint();
    assert_eq!(staged_deletion, repository.fingerprint());
    assert_eq!(
        BTreeSet::from([original, worktree_deletion, staged_deletion.clone()]).len(),
        3
    );
    repository.write("apps/module.rs", "pub fn verified() {}\n");
    assert_ne!(repository.fingerprint(), staged_deletion);
}

#[test]
fn staged_rename_binds_the_new_path_and_remains_deterministic() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    fs::rename(
        repository.root.join("apps/module.rs"),
        repository.root.join("apps/renamed.rs"),
    )
    .unwrap();
    let unstaged = repository.fingerprint();
    assert_ne!(unstaged, original);
    repository.git(&["add", "--all", "--", "apps"]);
    let staged = repository.fingerprint();
    assert_ne!(staged, original);
    assert_ne!(staged, unstaged);
    assert_eq!(staged, repository.fingerprint());
    repository.git(&["mv", "--", "apps/renamed.rs", "apps/module.rs"]);
    assert_eq!(repository.fingerprint(), original);
}

#[test]
fn additions_and_restoration_change_the_hash_and_staging_present_files_does_not() {
    let repository = Repository::new();
    repository.seed();
    let original = repository.fingerprint();
    repository.write("apps/addition.rs", "pub const NEW: bool = true;\n");
    let added = repository.fingerprint();
    assert_ne!(added, original);
    repository.git(&["add", "--", "apps/addition.rs"]);
    assert_eq!(repository.fingerprint(), added);
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    assert_ne!(repository.fingerprint(), added);
    repository.write("apps/module.rs", "pub fn verified() {}\n");
    assert_eq!(repository.fingerprint(), added);
}

#[test]
fn disappearance_or_restoration_after_inventory_is_rejected() {
    let repository = Repository::new();
    repository.seed();
    let inventory = source_inventory(&repository.root).unwrap();
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    let error = hash_source_inventory(&repository.root, &inventory).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("presence changed after inventory")
    );
    let deleted = source_inventory(&repository.root).unwrap();
    assert!(deleted.deleted.contains("apps/module.rs"));
    repository.write("apps/module.rs", "restored");
    let error = hash_source_inventory(&repository.root, &deleted).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("presence changed after inventory")
    );

    repository.write("apps/untracked.rs", "untracked");
    let untracked = source_inventory(&repository.root).unwrap();
    fs::remove_file(repository.root.join("apps/untracked.rs")).unwrap();
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
            "implementation": ["apps/module.rs"], "checks": ["source-check"],
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
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
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
    repository.write("apps/terrain.bin", "binary fixture");
    repository.git(&["add", "--", "apps/terrain.bin"]);
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
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    fs::create_dir(repository.root.join("apps/module.rs")).unwrap();
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
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    symlink("../Cargo.toml", repository.root.join("apps/module.rs")).unwrap();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );
    fs::remove_file(repository.root.join("apps/module.rs")).unwrap();
    symlink("missing-target", repository.root.join("apps/module.rs")).unwrap();
    assert!(
        source(&repository.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );

    let nested = Repository::new();
    nested.write("apps/source/module.rs", "tracked source");
    nested.git(&["add", "--", "apps/source/module.rs"]);
    fs::rename(
        nested.root.join("apps/source"),
        nested.root.join("relocated"),
    )
    .unwrap();
    symlink("../relocated", nested.root.join("apps/source")).unwrap();
    assert!(
        source(&nested.root)
            .unwrap_err()
            .to_string()
            .contains("symlink fingerprint input")
    );
}
