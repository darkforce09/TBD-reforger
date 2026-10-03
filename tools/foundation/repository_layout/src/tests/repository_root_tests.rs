use super::*;
use crate::documentation::{DOCUMENTATION_ROOT, GAP_ANALYSIS, ROADMAP};
use crate::{
    ARTIFACTS_DIR, BUILD_OUTPUT_FOLDER, CORPUS_PINS, CRF_FRAMEWORK_REFERENCE, ESTIMATES_DIR,
    ESTIMATES_SCHEMA, LAST_VERIFIED_MARKER, METRICS_DIR, METRICS_SCHEMA, QUEUE_JSON,
    REFERENCES_DIR, SCHEMA, SCOPE_VOCAB, TICKETS_DIR, VANILLA_REFERENCE, VERDICTS_DIR, WAVE_LOCK,
    WORKTREES_DIR,
};
use std::fs;

/// A scratch folder of its own under the system temporary folder, emptied first.
fn scratch(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("repository-layout-{name}-{}", std::process::id()));
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
        "tools/foundation/repository_layout",
        "tools/foundation/repository_layout/src",
    ] {
        let found = find_repository_root_from(&root.join(relative)).expect("repository root");
        assert_eq!(found, root, "from {relative:?}");
    }
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

#[test]
fn every_shared_location_is_relative_and_lies_under_its_tree() {
    let trees: [(&str, &[&str]); 4] = [
        (
            TICKETS_DIR,
            &[
                ROOT_MARKER,
                SCHEMA,
                SCOPE_VOCAB,
                CORPUS_PINS,
                WAVE_LOCK,
                QUEUE_JSON,
                METRICS_DIR,
                METRICS_SCHEMA,
                ESTIMATES_DIR,
                ESTIMATES_SCHEMA,
            ],
        ),
        (
            ARTIFACTS_DIR,
            &[WORKTREES_DIR, LAST_VERIFIED_MARKER, VERDICTS_DIR],
        ),
        (
            REFERENCES_DIR,
            &[CRF_FRAMEWORK_REFERENCE, VANILLA_REFERENCE],
        ),
        (DOCUMENTATION_ROOT, &[ROADMAP, GAP_ANALYSIS]),
    ];
    for (tree, locations) in trees {
        assert!(
            Path::new(tree).is_relative() && !tree.ends_with('/'),
            "{tree}"
        );
        for location in locations {
            assert!(
                location.starts_with(&format!("{tree}/")),
                "{location} lies outside {tree}"
            );
        }
    }
    assert!(!BUILD_OUTPUT_FOLDER.contains('/'));
}

#[test]
fn every_committed_shared_location_exists_in_this_checkout() {
    let root = find_repository_root().expect("repository root");
    let files = [
        ROOT_MARKER,
        SCHEMA,
        SCOPE_VOCAB,
        CORPUS_PINS,
        WAVE_LOCK,
        QUEUE_JSON,
        METRICS_SCHEMA,
        ESTIMATES_SCHEMA,
        ROADMAP,
        GAP_ANALYSIS,
    ];
    let folders = [
        TICKETS_DIR,
        ESTIMATES_DIR,
        ARTIFACTS_DIR,
        REFERENCES_DIR,
        DOCUMENTATION_ROOT,
    ];
    let missing: Vec<&str> = files
        .iter()
        .filter(|path| !root.join(path).is_file())
        .chain(folders.iter().filter(|path| !root.join(path).is_dir()))
        .copied()
        .collect();
    assert!(missing.is_empty(), "missing from the checkout: {missing:?}");
}
