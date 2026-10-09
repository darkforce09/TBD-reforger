//! The shared locations as a whole: each is relative and lies under the tree it belongs to, and
//! every committed one exists in this checkout.

use crate::documentation::{DOCUMENTATION_ROOT, GAP_ANALYSIS, ROADMAP};
use crate::{
    ARTIFACTS_DIR, BUILD_OUTPUT_FOLDER, CORPUS_PINS, CRF_FRAMEWORK_REFERENCE, ESTIMATES_DIR,
    ESTIMATES_SCHEMA, LAST_VERIFIED_MARKER, METRICS_DIR, METRICS_SCHEMA, QUEUE_JSON,
    REFERENCES_DIR, SCHEMA, SCOPE_VOCAB, TICKETS_DIR, VANILLA_REFERENCE, VERDICTS_DIR, WAVE_LOCK,
    WORKTREES_DIR,
};
use repository_root::{ROOT_MARKER, find_repository_root};
use std::path::Path;

/// The checkout-root marker is listed with the ticket registry's files: it lies in that tree.
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
