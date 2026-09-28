//! Tests for [`super`] — the law roots, their fail-closed walk and the test-file rule.

use super::*;
use crate::repository_laws::temporary_checkout::{TemporaryCheckout, this_repository};

#[test]
fn every_website_crate_src_and_tests_folder_joins_the_pins() {
    let checkout = TemporaryCheckout::with_pinned_roots("roots");
    checkout.write("apps/website/shared/src/cases.rs", "fn c() {}\n");
    checkout.write("apps/website/api_v2/tests/suite.rs", "fn t() {}\n");
    let roots = law_source_roots(checkout.root()).unwrap();
    let pinned: Vec<PathBuf> = FILE_LENGTH_PINS
        .iter()
        .map(|pin| checkout.root().join(pin))
        .collect();
    assert_eq!(roots[..FILE_LENGTH_PINS.len()], pinned[..]);
    assert!(roots.contains(&checkout.root().join("apps/website/shared/src")));
    assert!(roots.contains(&checkout.root().join("apps/website/api_v2/tests")));
    let pinned_src = checkout.root().join("apps/website/api_v2/src");
    assert_eq!(roots.iter().filter(|root| **root == pinned_src).count(), 1);
}

#[test]
fn a_missing_pin_is_a_walk_that_did_not_run() {
    for missing in FILE_LENGTH_PINS {
        let checkout = TemporaryCheckout::with_pinned_roots("missing-pin");
        std::fs::remove_dir_all(checkout.root().join(missing)).unwrap();
        assert!(
            matches!(
                walk_length_gated_sources(checkout.root()),
                Err(NotRun::TargetMissing(_))
            ),
            "missing {missing} must not read as an empty walk"
        );
    }
}

#[test]
fn the_walks_keep_only_their_extensions() {
    let checkout = TemporaryCheckout::with_pinned_roots("extensions");
    checkout.write("tools_v2/xtask/fixtures/TBD_Plant.c", "class A {}\n");
    checkout.write("tools_v2/xtask/fixtures/notes.txt", "notes\n");
    let gated = walk_length_gated_sources(checkout.root()).unwrap();
    assert_eq!(gated.len(), FILE_LENGTH_PINS.len() + 1);
    let rust = walk_rust_sources(checkout.root()).unwrap();
    assert_eq!(rust.len(), FILE_LENGTH_PINS.len());
    let everything = walk_law_sources(checkout.root(), |_| true).unwrap();
    assert_eq!(everything.len(), FILE_LENGTH_PINS.len() + 2);
}

#[test]
fn a_test_file_is_a_tests_component_or_a_tests_stem() {
    assert!(is_test_file("apps/website/api_v2/tests/integration.rs"));
    assert!(is_test_file("tools_v2/xtask/src/fixture_tests.rs"));
    assert!(is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash_tests.c"
    ));
    assert!(!is_test_file("tools_v2/xtask/src/test_helpers.rs"));
    assert!(!is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash.c"
    ));
    assert!(!is_test_file(
        "apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Hash_tests.h"
    ));
}

#[test]
fn only_the_shipped_addon_script_roots_may_be_pinned_under_apps_mod() {
    assert!(mod_pins_are_script_roots(
        &["tools_v2/xtask", "apps/mod/tbd-emcp/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
    assert!(!mod_pins_are_script_roots(&["apps/mod"], MOD_SCRIPT_ROOTS));
    assert!(!mod_pins_are_script_roots(
        &["apps/mod/vanilla_reference/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
    for reference in ["crf_framework", "vanilla_reference"] {
        assert!(
            MOD_SCRIPT_ROOTS
                .iter()
                .all(|root| !root.contains(reference))
        );
    }
}

#[test]
fn the_walk_over_this_repository_reads_every_pin() {
    let root = this_repository();
    let files = walk_length_gated_sources(&root).expect("the walk runs");
    let joined: Vec<String> = files
        .iter()
        .map(|path| repository_relative(&root, path))
        .collect();
    for pin in FILE_LENGTH_PINS {
        assert!(
            joined
                .iter()
                .any(|path| path.starts_with(&format!("{pin}/"))),
            "the walk read nothing under {pin}"
        );
    }
    assert!(joined.iter().any(|path| path.ends_with(".c")));
}
