//! The checkout-root walk: from the working directory and from any folder it stops at the nearest
//! marker file, agrees with the Cargo workspace root of this checkout, and refuses a walk that
//! reaches the filesystem root with the folder it started from and the marker it looked for.

use super::*;
use std::fs;

/// A scratch folder of its own under the system temporary folder, emptied first.
fn scratch(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("repository-root-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// Plants the root marker in `folder`, making it a checkout root.
fn plant_marker(folder: &Path) {
    let marker = folder.join(ROOT_MARKER);
    fs::create_dir_all(marker.parent().unwrap()).unwrap();
    fs::write(marker, "").unwrap();
}

#[test]
fn the_working_directory_walk_and_the_manifest_walk_find_the_same_checkout() {
    let from_working_directory = find_repository_root().expect("repository root");
    let from_manifest =
        find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("repository root");
    assert_eq!(from_working_directory, from_manifest);
    assert!(is_repository_root(&from_manifest));
}

#[test]
fn the_root_the_crate_folder_and_its_source_folder_resolve_to_one_root() {
    let root = find_repository_root().expect("repository root");
    for relative in [
        "",
        "crates/foundation/repository_root",
        "crates/foundation/repository_root/src",
    ] {
        let found = find_repository_root_from(&root.join(relative)).expect("repository root");
        assert_eq!(found, root, "from {relative:?}");
    }
}

/// The marker root of this checkout is the Cargo workspace root above the crate: the folder that
/// holds the lock file, the root manifest with its `[workspace]` table, and the captured API
/// responses the frontend goldens read.
#[test]
fn the_marker_root_is_the_cargo_workspace_root_above_the_crate() {
    let root = find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("root");
    assert!(root.join("Cargo.lock").is_file(), "{}", root.display());
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("the root manifest");
    assert!(
        manifest.lines().any(|line| line.trim() == "[workspace]"),
        "the root manifest declares the workspace"
    );
    assert!(
        root.join("contracts/fixtures/api_goldens").is_dir(),
        "{}",
        root.display()
    );
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR")).starts_with(&root),
        "the root is an ancestor of the manifest folder"
    );
}

#[test]
fn a_walk_stops_at_the_nearest_marker_so_a_nested_worktree_resolves_to_itself() {
    let outer = scratch("nested");
    let inner = outer.join("worktrees/slice");
    let deep = inner.join("tools/some_crate/src");
    fs::create_dir_all(&deep).unwrap();
    fs::create_dir_all(outer.join("documentation")).unwrap();
    plant_marker(&outer);
    plant_marker(&inner);
    assert_eq!(find_repository_root_from(&deep).unwrap(), inner);
    assert_eq!(find_repository_root_from(&inner).unwrap(), inner);
    assert_eq!(
        find_repository_root_from(&outer.join("documentation")).unwrap(),
        outer
    );
    fs::remove_dir_all(&outer).unwrap();
}

/// A throwaway Cargo workspace (a lock file and a `[workspace]` manifest, as the law tests build)
/// is no checkout root without the marker, so a walk from inside it passes it by.
#[test]
fn a_cargo_workspace_without_the_marker_is_passed_by() {
    let outer = scratch("cargo-workspace");
    let workspace = outer.join("throwaway");
    let deep = workspace.join("crates/member/src");
    fs::create_dir_all(&deep).unwrap();
    fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    fs::write(workspace.join("Cargo.lock"), "version = 4\n").unwrap();
    plant_marker(&outer);
    assert!(!is_repository_root(&workspace));
    assert_eq!(find_repository_root_from(&deep).unwrap(), outer);
    fs::remove_dir_all(&outer).unwrap();
}

#[test]
fn a_folder_without_the_marker_file_is_not_a_root() {
    let folder = scratch("unmarked");
    assert!(!is_repository_root(&folder));
    fs::create_dir_all(folder.join(ROOT_MARKER)).unwrap();
    assert!(
        !is_repository_root(&folder),
        "a folder named like the marker is not the marker file"
    );
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn a_walk_that_reaches_the_filesystem_root_is_an_error_naming_its_start_and_the_marker() {
    let error = find_repository_root_from(Path::new("/")).unwrap_err();
    assert!(matches!(&error, Error::RootMarkerNotFound { start } if start == Path::new("/")));
    let message = error.to_string();
    assert!(message.contains(ROOT_MARKER), "{message}");
    assert!(message.contains("from / upward"), "{message}");
}

/// The system temporary folder lies outside every checkout, so a walk from it is refused with the
/// folder it searched, never answered with a guess.
#[test]
fn a_folder_outside_any_checkout_has_no_root() {
    let outside = std::env::temp_dir();
    let error = find_repository_root_from(&outside).unwrap_err();
    assert!(matches!(&error, Error::RootMarkerNotFound { start } if *start == outside));
    let message = error.to_string();
    assert!(
        message.contains("could not find the repository root"),
        "{message}"
    );
    assert!(
        message.contains(&outside.display().to_string()),
        "{message}"
    );
}

#[test]
fn the_marker_is_a_relative_file_path_of_this_checkout() {
    assert!(Path::new(ROOT_MARKER).is_relative());
    assert!(!ROOT_MARKER.ends_with('/'));
    let root = find_repository_root().expect("repository root");
    assert!(root.join(ROOT_MARKER).is_file(), "{}", root.display());
}
