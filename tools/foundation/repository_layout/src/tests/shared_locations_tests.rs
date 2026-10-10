//! The shared locations as a whole: each is relative and lies under the tree it belongs to, and
//! every committed one exists in this checkout.

use crate::documentation::DOCUMENTATION_ROOT;
use crate::{
    BUILD_OUTPUT_FOLDER, CRF_FRAMEWORK_REFERENCE, ENFUSION_MCP_GAME_ROOT, LAST_VERIFIED_MARKER,
    PLAYTEST_SERVER_DIR, REFERENCES_DIR, REFORGER_EXTRACT_DIR, VANILLA_REFERENCE, VERDICTS_DIR,
    WORKSTATION_DIR, WORKSTATION_LOGS_DIR, WORKTREES_DIR,
};
use repository_root::{ROOT_MARKER, find_repository_root};
use std::path::Path;

#[test]
fn every_shared_location_is_relative_and_lies_under_its_tree() {
    let trees: [(&str, &[&str]); 2] = [
        (
            WORKSTATION_DIR,
            &[
                LAST_VERIFIED_MARKER,
                VERDICTS_DIR,
                WORKSTATION_LOGS_DIR,
                ENFUSION_MCP_GAME_ROOT,
                PLAYTEST_SERVER_DIR,
                REFORGER_EXTRACT_DIR,
            ],
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
    for top_level in [
        WORKSTATION_DIR,
        WORKTREES_DIR,
        ROOT_MARKER,
        BUILD_OUTPUT_FOLDER,
    ] {
        assert!(!top_level.contains('/'), "{top_level} is not top-level");
    }
    assert!(Path::new(DOCUMENTATION_ROOT).is_relative() && !DOCUMENTATION_ROOT.ends_with('/'));
}

#[test]
fn every_committed_shared_location_exists_in_this_checkout() {
    let root = find_repository_root().expect("repository root");
    let files = [ROOT_MARKER];
    let folders = [REFERENCES_DIR, DOCUMENTATION_ROOT];
    let missing: Vec<&str> = files
        .iter()
        .filter(|path| !root.join(path).is_file())
        .chain(folders.iter().filter(|path| !root.join(path).is_dir()))
        .copied()
        .collect();
    assert!(missing.is_empty(), "missing from the checkout: {missing:?}");
}
