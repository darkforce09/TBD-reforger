//! The shared locations as a whole: each is relative and lies under the tree it belongs to, and
//! every committed one exists in this checkout.

use crate::documentation::DOCUMENTATION_ROOT;
use crate::{
    ARTIFACTS_DIR, BUILD_OUTPUT_FOLDER, CRF_FRAMEWORK_REFERENCE, LAST_VERIFIED_MARKER,
    LEGACY_TICKETS_DIR, REFERENCES_DIR, VANILLA_REFERENCE, VERDICTS_DIR, WORKTREES_DIR,
};
use repository_root::{ROOT_MARKER, find_repository_root};
use std::path::Path;

/// The agent folder that holds the checkout-root marker, the artifact tree and the legacy ticket
/// data.
const AGENT_FOLDER: &str = ".ai";

#[test]
fn every_shared_location_is_relative_and_lies_under_its_tree() {
    let trees: [(&str, &[&str]); 3] = [
        (
            AGENT_FOLDER,
            &[ROOT_MARKER, ARTIFACTS_DIR, LEGACY_TICKETS_DIR],
        ),
        (
            ARTIFACTS_DIR,
            &[WORKTREES_DIR, LAST_VERIFIED_MARKER, VERDICTS_DIR],
        ),
        (
            REFERENCES_DIR,
            &[CRF_FRAMEWORK_REFERENCE, VANILLA_REFERENCE],
        ),
    ];
    for (tree, locations) in trees {
        assert!(
            Path::new(tree).is_relative() && !tree.ends_with('/'),
            "{tree}"
        );
        for location in locations {
            assert!(
                location.starts_with(&format!("{tree}/")) && !location.ends_with('/'),
                "{location} lies outside {tree}"
            );
        }
    }
    assert!(!BUILD_OUTPUT_FOLDER.contains('/'));
    assert!(Path::new(DOCUMENTATION_ROOT).is_relative() && !DOCUMENTATION_ROOT.ends_with('/'));
}

#[test]
fn every_committed_shared_location_exists_in_this_checkout() {
    let root = find_repository_root().expect("repository root");
    let files = [ROOT_MARKER];
    let folders = [ARTIFACTS_DIR, REFERENCES_DIR, DOCUMENTATION_ROOT];
    let missing: Vec<&str> = files
        .iter()
        .filter(|path| !root.join(path).is_file())
        .chain(folders.iter().filter(|path| !root.join(path).is_dir()))
        .copied()
        .collect();
    assert!(missing.is_empty(), "missing from the checkout: {missing:?}");
}
