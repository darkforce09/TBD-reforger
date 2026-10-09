//! Unit tests for [`crate::frontend_package_lane`]: the derived frontend family over fixture
//! workspaces and this checkout, and the rendered lines.

use super::*;

/// An empty fixture folder under the system temp folder, unique to this process and `name`.
fn fixture_root(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "frontend-package-lane-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture root");
    dir
}

/// A workspace under `root` with the per-category member globs, and one package manifest per
/// `(path, package)`.
fn workspace(root: &Path, members: &[(&str, &str)]) {
    let write = |relative: &str, body: &str| {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    };
    write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"apps/*\", \"crates/foundation/*\", \"crates/frontend/*/*\", \
         \"crates/frontend_tools/*\"]\n",
    );
    for (path, package) in members {
        write(
            &format!("{path}/Cargo.toml"),
            &format!("[package]\nname = \"{package}\"\n"),
        );
    }
}

/// The app heads the family, then every crate under `crates/frontend` in path order, the offline
/// service worker beside the app among them; members elsewhere, a `crates/frontend_tools`
/// look-alike among them, stay out.
#[test]
fn the_family_is_the_app_then_every_frontend_crate_in_path_order() {
    let root = fixture_root("family");
    workspace(
        &root,
        &[
            ("apps/server", "api"),
            (
                "crates/frontend/shell/frontend_application",
                "frontend_application",
            ),
            ("crates/foundation/browser_platform", "browser_platform"),
            ("crates/frontend/pages/operations_pages", "operations_pages"),
            (
                "crates/frontend/shell/offline_service_worker",
                "offline_service_worker",
            ),
            ("crates/frontend/foundation/frontend_ui", "frontend_ui"),
            (
                "crates/frontend/foundation/frontend_api_dtos",
                "frontend_api_dtos",
            ),
            ("crates/frontend/workspaces/debug_benches", "debug_benches"),
            ("crates/frontend_tools/look_alike", "look_alike"),
        ],
    );
    let packages = frontend_packages(&root).expect("the fixture reads");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        packages,
        [
            "frontend_application",
            "frontend_api_dtos",
            "frontend_ui",
            "operations_pages",
            "offline_service_worker",
            "debug_benches",
        ]
        .map(String::from)
        .to_vec()
    );
}

/// A crate born under `crates/frontend` joins every line without an edit to any list.
#[test]
fn a_born_frontend_crate_joins_every_line() {
    let root = fixture_root("born");
    workspace(
        &root,
        &[(
            "crates/frontend/shell/frontend_application",
            "frontend_application",
        )],
    );
    let before = frontend_line_argv(&root, FrontendLine::Test).expect("the fixture reads");
    workspace(
        &root,
        &[
            (
                "crates/frontend/shell/frontend_application",
                "frontend_application",
            ),
            ("crates/frontend/pages/zz_probe_pages", "zz_probe_pages"),
        ],
    );
    let after: Vec<String> = [
        FrontendLine::Format,
        FrontendLine::Wasm32Clippy,
        FrontendLine::NativeClippy,
        FrontendLine::Test,
    ]
    .into_iter()
    .map(|line| {
        frontend_line_argv(&root, line)
            .expect("the fixture reads")
            .join(" ")
    })
    .collect();
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(before.join(" "), "cargo test -p frontend_application");
    for line in &after {
        assert!(
            format!("{line} ").contains("-p frontend_application -p zz_probe_pages "),
            "the born crate is not named: {line}"
        );
    }
}

/// A workspace without the app is refused, never a family of crates alone.
#[test]
fn a_workspace_without_the_app_is_refused() {
    let root = fixture_root("no-app");
    workspace(
        &root,
        &[("crates/frontend/foundation/frontend_ui", "frontend_ui")],
    );
    let refused = frontend_packages(&root).expect_err("no app is refused");
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        refused.to_string().contains("`frontend_application`"),
        "{refused}"
    );
}

/// A folder with no root manifest is an error, never an empty family.
#[test]
fn an_unreadable_workspace_is_an_error_not_an_empty_family() {
    let missing = fixture_root("unreadable");
    let refused = frontend_packages(&missing).expect_err("no root manifest is refused");
    let _ = std::fs::remove_dir_all(&missing);
    assert!(refused.to_string().contains("Cargo.toml"), "{refused}");
}

/// Each line names every package, then its own words, with the flags of the ci.yml frontend job.
#[test]
fn each_line_names_every_package_then_its_own_words() {
    let packages = ["frontend_application", "frontend_ui"]
        .map(String::from)
        .to_vec();
    let rendered = |line| frontend_cargo_argv(line, &packages).join(" ");
    assert_eq!(
        rendered(FrontendLine::Format),
        "cargo fmt -p frontend_application -p frontend_ui --check"
    );
    assert_eq!(
        rendered(FrontendLine::Wasm32Clippy),
        "cargo clippy -p frontend_application -p frontend_ui --target wasm32-unknown-unknown \
         --all-targets -- -D warnings"
    );
    assert_eq!(
        rendered(FrontendLine::NativeClippy),
        "cargo clippy -p frontend_application -p frontend_ui --all-targets --locked -- -D warnings"
    );
    assert_eq!(
        rendered(FrontendLine::Test),
        "cargo test -p frontend_application -p frontend_ui"
    );
}

/// A caller's own words wrap the family's `-p` list unchanged.
#[test]
fn the_family_argv_wraps_the_callers_words() {
    let root = fixture_root("caller-words");
    workspace(
        &root,
        &[
            (
                "crates/frontend/shell/frontend_application",
                "frontend_application",
            ),
            ("crates/frontend/foundation/frontend_ui", "frontend_ui"),
        ],
    );
    let argv = frontend_family_argv(&root, &["env", "A=1", "cargo", "test"], &["--quiet"])
        .expect("the fixture reads");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        argv.join(" "),
        "env A=1 cargo test -p frontend_application -p frontend_ui --quiet"
    );
}

/// Over this checkout the family is headed by the app and holds exactly the members under
/// `crates/frontend`, the app and the offline service worker among them, each once.
#[test]
fn this_checkout_family_is_the_app_and_the_frontend_crates() {
    let root = tool_test_support::test_repo_root();
    let members = read_workspace_members(&root).expect("the workspace members read");
    let packages = frontend_packages(&root).expect("the frontend family derives");
    assert_eq!(
        packages.first().map(String::as_str),
        Some(FRONTEND_APPLICATION)
    );
    for package in &packages {
        assert_eq!(
            packages.iter().filter(|other| *other == package).count(),
            1,
            "{package} is named twice: {packages:?}"
        );
    }
    assert!(
        packages
            .iter()
            .any(|package| package == "offline_service_worker"),
        "{packages:?}"
    );
    let expected: Vec<&str> = members
        .iter()
        .filter(|member| member.path.starts_with("crates/frontend/"))
        .map(|member| member.package_name.as_str())
        .filter(|package| *package != FRONTEND_APPLICATION)
        .collect();
    let crates: Vec<&str> = packages[1..].iter().map(String::as_str).collect();
    assert_eq!(crates, expected);
}
