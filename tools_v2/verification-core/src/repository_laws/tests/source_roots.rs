//! Tests for [`super`] — the law roots, their fail-closed walk and the test-file rule.

use super::*;
use crate::repository_laws::temporary_checkout::{
    FIXTURE_WORKSPACE_MEMBERS, TemporaryCheckout, this_repository,
};
use crate::repository_laws::workspace_members::read_workspace_members;

#[test]
fn source_roots_every_missing_root_is_a_walk_that_did_not_run() {
    for missing in FIXTURE_WORKSPACE_MEMBERS.iter().chain(PINNED_SCRIPT_ROOTS) {
        let checkout = TemporaryCheckout::with_law_roots("missing-root");
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
fn source_roots_the_walks_keep_only_their_extensions() {
    let checkout = TemporaryCheckout::with_law_roots("extensions");
    checkout.write("tools_v2/xtask/fixtures/TBD_Plant.c", "class A {}\n");
    checkout.write("tools_v2/xtask/fixtures/notes.txt", "notes\n");
    let members = FIXTURE_WORKSPACE_MEMBERS.len();
    let gated = walk_length_gated_sources(checkout.root()).unwrap();
    assert_eq!(gated.len(), members + 1);
    let rust = walk_rust_sources(checkout.root()).unwrap();
    assert_eq!(rust.len(), members);
    let everything = walk_law_sources(checkout.root(), |_| true).unwrap();
    assert_eq!(
        everything.len(),
        2 * members + 2,
        "a lib.rs and a Cargo.toml per member"
    );
}

#[test]
fn source_roots_a_test_file_is_a_tests_component_or_a_tests_stem() {
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
fn source_roots_only_the_shipped_addon_script_roots_may_be_pinned_under_apps_mod() {
    assert!(mod_pins_are_script_roots(
        &["tools_v2/xtask", "apps/mod/tbd-emcp/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
    assert!(!mod_pins_are_script_roots(&["apps/mod"], MOD_SCRIPT_ROOTS));
    assert!(!mod_pins_are_script_roots(
        &["apps/mod/vanilla_reference/Scripts"],
        MOD_SCRIPT_ROOTS
    ));
    assert!(mod_pins_are_script_roots(
        PINNED_SCRIPT_ROOTS,
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
fn source_roots_the_walk_over_this_repository_reads_every_member_and_script_root() {
    let root = this_repository();
    let files = walk_length_gated_sources(&root).expect("the walk runs");
    let joined: Vec<String> = files
        .iter()
        .map(|path| repository_relative(&root, path))
        .collect();
    let members = read_workspace_members(&root).expect("the root manifest reads");
    assert!(!members.is_empty());
    let member_folders = members.iter().map(|member| member.path.as_str());
    for folder in member_folders.chain(PINNED_SCRIPT_ROOTS.iter().copied()) {
        assert!(
            joined
                .iter()
                .any(|path| path.starts_with(&format!("{folder}/"))),
            "the walk read nothing under {folder}"
        );
    }
    assert!(joined.iter().any(|path| path.ends_with(".c")));
}

#[test]
fn source_roots_outermost_folders_drop_every_contained_folder() {
    assert_eq!(
        outermost_folders(&[
            "tools/outer",
            "tools/outer/inner",
            "tools/outer_sibling",
            "crates/a"
        ]),
        ["tools/outer", "tools/outer_sibling", "crates/a"]
    );
}

const SCRIPT_ROOTS: &[&str] = PINNED_SCRIPT_ROOTS;

/// A checkout whose root manifest lists `entries` as members, with a crate in each of
/// `member_folders` and one script in each pinned script root.
fn member_workspace(name: &str, entries: &[&str], member_folders: &[&str]) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(&format!("source-roots-{name}"));
    let listed: Vec<String> = entries
        .iter()
        .map(|entry| format!("    \"{entry}\",\n"))
        .collect();
    checkout.write(
        "Cargo.toml",
        &format!("[workspace]\nmembers = [\n{}]\n", listed.concat()),
    );
    for folder in member_folders {
        let name = folder.rsplit('/').next().unwrap();
        checkout.write(
            &format!("{folder}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\n"),
        );
        checkout.write(&format!("{folder}/src/lib.rs"), "fn placeholder() {}\n");
    }
    for root in SCRIPT_ROOTS {
        checkout.write(&format!("{root}/Game/TBD_Plant.c"), "class TBD_Plant {}\n");
    }
    checkout
}

fn relative_files(checkout: &TemporaryCheckout, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|file| repository_relative(checkout.root(), file))
        .collect()
}

#[test]
fn source_roots_follow_the_workspace_members() {
    let members = [
        "crates/foundation/ids",
        "crates/mission/mission_model",
        "tools/staging/fixture_tool",
    ];
    let checkout = member_workspace(
        "follow",
        &["crates/*/*", "tools/staging/fixture_tool"],
        &members,
    );
    checkout.write("scratch/loose/src/stray.rs", "fn stray() {}\n");
    checkout.write(
        "crates/foundation/no_manifest/src/stray.rs",
        "fn stray() {}\n",
    );
    let roots = law_source_roots(checkout.root()).unwrap();
    let expected: Vec<PathBuf> = members
        .iter()
        .chain(SCRIPT_ROOTS)
        .map(|folder| checkout.root().join(folder))
        .collect();
    assert_eq!(roots, expected);
    let rust = relative_files(&checkout, &walk_rust_sources(checkout.root()).unwrap());
    let expected_rust: Vec<String> = members
        .iter()
        .map(|folder| format!("{folder}/src/lib.rs"))
        .collect();
    assert_eq!(rust, expected_rust);
}

#[test]
fn source_roots_a_deleted_member_folder_is_target_missing() {
    let members = ["crates/foundation/ids", "tools/staging/fixture_tool"];
    let checkout = member_workspace("deleted", &members, &members);
    assert!(walk_length_gated_sources(checkout.root()).is_ok());
    let deleted = checkout.root().join("tools/staging/fixture_tool");
    std::fs::remove_dir_all(&deleted).unwrap();
    for outcome in [
        law_source_roots(checkout.root()).map(|_| ()),
        walk_length_gated_sources(checkout.root()).map(|_| ()),
    ] {
        assert!(
            matches!(&outcome, Err(NotRun::TargetMissing(path)) if *path == deleted),
            "a deleted member must not read as a smaller walk: {outcome:?}"
        );
    }
    let no_members = member_workspace("no-members", &[], &[]);
    assert!(matches!(
        law_source_roots(no_members.root()),
        Err(NotRun::TargetMissing(_))
    ));
    let no_manifest = TemporaryCheckout::empty("source-roots-no-manifest");
    assert!(matches!(
        law_source_roots(no_manifest.root()),
        Err(NotRun::TargetMissing(_))
    ));
}

#[test]
fn source_roots_still_walk_the_pinned_mod_script_roots() {
    let members = ["crates/foundation/ids"];
    let checkout = member_workspace("scripts", &members, &members);
    checkout.write(
        "apps/mod/tbd-export/Scripts/Game/TBD_Export.c",
        "class TBD_Export {}\n",
    );
    checkout.write(
        "apps/mod/vanilla_reference/Scripts/Game/Vanilla.c",
        "class Vanilla {}\n",
    );
    let gated = relative_files(
        &checkout,
        &walk_length_gated_sources(checkout.root()).unwrap(),
    );
    let mut expected: Vec<String> = SCRIPT_ROOTS
        .iter()
        .map(|root| format!("{root}/Game/TBD_Plant.c"))
        .collect();
    expected.push("crates/foundation/ids/src/lib.rs".to_string());
    expected.sort();
    assert_eq!(gated, expected);
    for root in SCRIPT_ROOTS {
        let checkout = member_workspace("scripts-missing", &members, &members);
        std::fs::remove_dir_all(checkout.root().join(root)).unwrap();
        assert!(
            matches!(
                walk_length_gated_sources(checkout.root()),
                Err(NotRun::TargetMissing(_))
            ),
            "missing {root} must not read as an empty walk"
        );
    }
}

#[test]
fn source_roots_a_nested_member_is_walked_once() {
    let members = ["tools/outer", "tools/outer/inner"];
    let checkout = member_workspace("nested", &members, &members);
    let roots = law_source_roots(checkout.root()).unwrap();
    assert!(roots.contains(&checkout.root().join("tools/outer")));
    assert!(!roots.contains(&checkout.root().join("tools/outer/inner")));
    let rust = relative_files(&checkout, &walk_rust_sources(checkout.root()).unwrap());
    assert_eq!(
        rust,
        ["tools/outer/inner/src/lib.rs", "tools/outer/src/lib.rs"]
    );
}
