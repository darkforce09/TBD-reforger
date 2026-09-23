use super::*;
use std::collections::BTreeSet;
use verification_core::NotRun;
use verification_core::proc::{Output, Run};

fn names(set: &BTreeSet<String>) -> Vec<&str> {
    set.iter().map(String::as_str).collect()
}

#[test]
fn a_listing_yields_its_files_and_every_folder_above_them() {
    let tree = TrackedTree::from_listing("apps/a/b/c.rs\0apps/a/README.md\0top.md\0");
    assert_eq!(
        tree.files().collect::<Vec<_>>(),
        ["apps/a/README.md", "apps/a/b/c.rs", "top.md"]
    );
    assert_eq!(
        tree.folders().collect::<Vec<_>>(),
        ["apps", "apps/a", "apps/a/b"]
    );
    assert_eq!(tree.file_count(), 3);

    let root = tree.children("").expect("the repository root");
    assert_eq!(names(&root.files), ["top.md"]);
    assert_eq!(names(&root.folders), ["apps"]);
    let a = tree.children("apps/a").expect("apps/a");
    assert_eq!(names(&a.files), ["README.md"]);
    assert_eq!(names(&a.folders), ["b"]);
    assert_eq!(
        names(&tree.children("apps/a/b").expect("apps/a/b").files),
        ["c.rs"]
    );
}

#[test]
fn files_and_folders_are_told_apart_and_only_tracked_ones_exist() {
    let tree = TrackedTree::from_listing("apps/a/b/c.rs");
    assert!(tree.is_file("apps/a/b/c.rs"));
    assert!(!tree.is_folder("apps/a/b/c.rs"));
    assert!(tree.is_folder("apps/a/b"));
    assert!(!tree.is_file("apps/a/b"));
    assert!(
        tree.is_folder(""),
        "the repository root holds every tracked file"
    );
    assert!(!tree.is_folder("apps/a/c"));
    assert!(tree.children("apps/x").is_none());
}

#[test]
fn siblings_share_their_folders_without_duplicates() {
    let tree = TrackedTree::from_listing("a/b/one.rs\0a/b/two.rs\0a/c/three.rs\0a/four.rs");
    let a = tree.children("a").expect("a");
    assert_eq!(names(&a.files), ["four.rs"]);
    assert_eq!(names(&a.folders), ["b", "c"]);
    assert_eq!(
        names(&tree.children("a/b").expect("a/b").files),
        ["one.rs", "two.rs"]
    );
}

#[test]
fn an_empty_listing_is_an_empty_tree() {
    let tree = TrackedTree::from_listing("");
    assert_eq!(tree.file_count(), 0);
    assert_eq!(tree.folders().count(), 0);
    assert!(!tree.is_folder(""));
}

#[test]
fn names_with_spaces_survive_the_listing() {
    let tree = TrackedTree::from_listing("apps/With space/README.md\0");
    assert!(tree.is_folder("apps/With space"));
    assert!(tree.is_file("apps/With space/README.md"));
}

#[test]
fn a_listing_that_exits_non_zero_did_not_run() {
    let output = Output {
        code: 128,
        stdout: String::new(),
        stderr: "fatal: not a git repository\n".to_string(),
        duration: std::time::Duration::ZERO,
    };
    match from_listing_output(output) {
        Err(NotRun::ToolError {
            tool,
            status,
            stderr,
        }) => {
            assert_eq!(tool, "git ls-files -z");
            assert_eq!(status, 128);
            assert_eq!(stderr, "fatal: not a git repository");
        }
        other => panic!("a failed listing must not run: {other:?}"),
    }
}

#[test]
fn a_listing_that_exits_zero_is_the_tree_it_printed() {
    let output = Output {
        code: 0,
        stdout: "a/b.rs\0".to_string(),
        stderr: String::new(),
        duration: std::time::Duration::ZERO,
    };
    let tree = from_listing_output(output).expect("a clean listing");
    assert!(tree.is_file("a/b.rs"));
}

#[test]
fn a_missing_listing_program_did_not_run() {
    match read_listing(Run::new("tbd-no-such-listing-program")) {
        Err(NotRun::ToolAbsent(program)) => assert_eq!(program, "tbd-no-such-listing-program"),
        other => panic!("an absent program must not run: {other:?}"),
    }
}

#[test]
fn a_git_that_refuses_its_arguments_did_not_run() {
    let refused = read_listing(Run::new("git").args(["ls-files", "--no-such-option"]));
    match refused {
        Err(NotRun::ToolError { status, .. }) => assert_ne!(status, 0),
        other => panic!("a refused listing must not run: {other:?}"),
    }
}

#[test]
fn the_checkout_lists_its_committed_layout() {
    use crate::core::repository_layout::{DEPLOY_DIR, DEPLOY_ENV_EXAMPLE};
    let root = crate::core::repository_root::test_repo_root();
    let tree = TrackedTree::load(&root).expect("git ls-files runs in the checkout");
    assert!(
        tree.file_count() > 1000,
        "too few tracked files: {}",
        tree.file_count()
    );
    assert!(tree.is_file(DEPLOY_ENV_EXAMPLE));
    assert!(tree.is_folder(DEPLOY_DIR));
}
