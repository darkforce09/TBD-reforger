//! Unit tests for [`crate::workspace_member_tests`]: the derived package list over
//! fixture workspaces.

use super::*;

fn fixture_root(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "workspace-member-tests-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture root");
    dir
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

/// A workspace of `members` (`(path, package)`) under `root`, with a glob member entry.
fn workspace(root: &Path, members: &[(&str, &str)]) {
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"apps/*\", \"crates/*/*\"]\n",
    );
    for (path, package) in members {
        write(
            root,
            &format!("{path}/Cargo.toml"),
            &format!("[package]\nname = \"{package}\"\n"),
        );
    }
}

#[test]
fn every_member_outside_the_dedicated_packages_is_listed() {
    let root = fixture_root("listed");
    workspace(
        &root,
        &[
            ("apps/api", "api"),
            ("apps/agent", "agent"),
            ("crates/foundation/guard", "guard"),
        ],
    );
    assert_eq!(
        member_packages_except(&root, &["api"]).expect("the fixture reads"),
        vec!["agent".to_string(), "guard".to_string()],
        "a member the glob finds is tested without a list naming it"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_dedicated_package_that_is_no_member_is_an_error() {
    let root = fixture_root("stale");
    workspace(&root, &[("apps/api", "api")]);
    let error = member_packages_except(&root, &["api", "renamed_away"])
        .expect_err("a stale dedicated entry");
    assert!(
        error.to_string().contains("renamed_away"),
        "the error names the stale entry: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_workspace_is_an_error_not_an_empty_lane() {
    let root = fixture_root("unreadable");
    write(&root, "apps/api/Cargo.toml", "[package]\nname = \"api\"\n");
    let error = member_packages_except(&root, &[]).expect_err("no root manifest");
    assert!(
        error.to_string().contains("Cargo.toml"),
        "the error names the manifest: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The API crates leave this lane with `api`: `api-test` and `db test-it` test the API family
/// together, so a crate under `crates/api` is never tested twice nor dropped from both.
#[test]
fn the_api_family_is_left_to_the_api_lane() {
    let root = fixture_root("api-family");
    workspace(
        &root,
        &[
            ("apps/api", "api"),
            ("apps/frontend", "frontend"),
            ("apps/offline_service_worker", "offline_service_worker"),
            ("apps/agent", "agent"),
            ("crates/api/api_state", "api_state"),
            ("crates/foundation/guard", "guard"),
        ],
    );
    assert_eq!(
        workspace_member_lane_packages(&root).expect("the fixture reads"),
        vec!["agent".to_string(), "guard".to_string()]
    );
    let _ = std::fs::remove_dir_all(&root);
}
