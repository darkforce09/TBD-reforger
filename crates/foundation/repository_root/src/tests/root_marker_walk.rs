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
fn a_walk_that_reaches_the_filesystem_root_is_an_error_naming_its_start_and_the_marker() {
    let error = find_repository_root_from(Path::new("/")).unwrap_err();
    assert!(matches!(&error, Error::RootMarkerNotFound { start } if start == Path::new("/")));
    let message = error.to_string();
    assert!(message.contains(ROOT_MARKER), "{message}");
    assert!(message.contains("from / upward"), "{message}");
}
