//! Unit tests for [`crate::local_database::api_test_packages`]: the derived API package list over
//! fixture workspaces and over this checkout.

use super::*;

fn fixture_root(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("api-test-packages-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture root");
    dir
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

/// A workspace of `members` (`(path, package)`) under `root`, with glob member entries.
fn workspace(root: &Path, members: &[(&str, &str)]) {
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"apps/*\", \"crates/*/*\", \"tools/*/*\"]\n",
    );
    for (path, package) in members {
        write(
            root,
            &format!("{path}/Cargo.toml"),
            &format!("[package]\nname = \"{package}\"\n"),
        );
    }
}

/// Appends the dependency `table` (`dependencies`, `dev-dependencies`, …) naming `dependency`
/// to the manifest of the member at `path`.
fn depend(root: &Path, path: &str, table: &str, dependency: &str) {
    let manifest = root.join(path).join("Cargo.toml");
    let mut body = std::fs::read_to_string(&manifest).expect("member manifest");
    body.push_str(&format!(
        "\n[{table}]\n{dependency} = {{ workspace = true }}\n"
    ));
    std::fs::write(manifest, body).expect("write");
}

#[test]
fn the_api_app_comes_first_then_every_api_crate_and_nothing_else() {
    let root = fixture_root("listed");
    workspace(
        &root,
        &[
            ("apps/api", "api"),
            ("apps/agent", "agent"),
            ("crates/api/api_state", "api_state"),
            ("crates/api/api_database", "api_database"),
            ("crates/foundation/guard", "guard"),
        ],
    );
    assert_eq!(
        api_test_packages(&root).expect("the fixture reads"),
        ["api", "api_database", "api_state"],
        "a crate the glob finds under crates/api is covered without a list naming it"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_workspace_without_the_api_package_is_an_error() {
    let root = fixture_root("no-api");
    workspace(&root, &[("crates/api/api_state", "api_state")]);
    let error = api_test_packages(&root).expect_err("no api member");
    assert!(error.to_string().contains("`api`"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_workspace_is_an_error_not_an_api_only_lane() {
    let root = fixture_root("unreadable");
    write(&root, "apps/api/Cargo.toml", "[package]\nname = \"api\"\n");
    let error = api_test_packages(&root).expect_err("no root manifest");
    assert!(error.to_string().contains("Cargo.toml"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn package_arguments_pair_each_package_with_its_flag() {
    let packages = ["api".to_string(), "api_state".to_string()];
    assert_eq!(
        package_arguments(&packages),
        ["-p", "api", "-p", "api_state"]
    );
    assert!(package_arguments(&[]).is_empty());
}

#[test]
fn a_member_that_uses_an_api_crate_follows_the_api_crates() {
    let root = fixture_root("users");
    workspace(
        &root,
        &[
            ("apps/api", "api"),
            ("apps/agent", "agent"),
            ("crates/api/api_state", "api_state"),
            ("crates/foundation/guard", "guard"),
            ("tools/staging/fixtures_tool", "fixtures_tool"),
            ("tools/staging/probe_tool", "probe_tool"),
            ("tools/commands/builder", "builder"),
        ],
    );
    depend(&root, "apps/agent", "dependencies", "guard");
    depend(&root, "crates/api/api_state", "dependencies", "guard");
    depend(
        &root,
        "tools/staging/fixtures_tool",
        "dependencies",
        "api_state",
    );
    depend(
        &root,
        "tools/staging/probe_tool",
        "dev-dependencies",
        "api_state",
    );
    depend(
        &root,
        "tools/commands/builder",
        "build-dependencies",
        "api_state",
    );
    assert_eq!(
        api_test_packages(&root).expect("the fixture reads"),
        ["api", "api_state", "fixtures_tool", "probe_tool"],
        "a member whose build or tests use an API crate is covered after the API crates, in \
         member-path order; a build-script edge alone is not a use"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The live checkout: every `crates/api/<crate>/Cargo.toml` on disk is one derived package, and
/// the staging fixtures tool, which writes through the API's services, follows them.
#[test]
fn this_checkout_derives_every_api_crate_on_disk_and_the_staging_fixtures_tool() {
    let root = tool_test_support::test_repo_root();
    let derived = api_test_packages(&root).expect("this checkout's workspace reads");
    let on_disk = std::fs::read_dir(root.join(API_CRATES_FOLDER))
        .expect("crates/api exists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .count();
    assert!(on_disk > 0, "crates/api holds no crate");
    assert_eq!(derived[0], API_APPLICATION_PACKAGE);
    assert!(
        derived[1..=on_disk]
            .iter()
            .all(|package| package.starts_with("api_")),
        "{derived:?}"
    );
    assert_eq!(
        &derived[on_disk + 1..],
        ["staging_fixtures"],
        "the members outside crates/api that use an API crate: {derived:?}"
    );
}
