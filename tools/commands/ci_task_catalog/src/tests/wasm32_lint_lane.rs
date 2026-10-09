//! Unit tests for [`crate::wasm32_lint_lane`]: the derived package set over this checkout, its
//! split between the two lanes that lint it, the frontend crates and the refusals over fixture
//! workspaces.

use super::*;
use repository_laws::workspace_members::read_workspace_members;

/// The checkout the test binary was built from, whatever the working directory.
fn root() -> std::path::PathBuf {
    tool_test_support::test_repo_root()
}

/// The packages of this checkout whose layout table declares `targets = "wasm32"`, read straight
/// from the members' manifests.
fn declared_wasm32_packages() -> Vec<String> {
    read_workspace_members(&root())
        .expect("the workspace members read")
        .into_iter()
        .filter(|member| {
            member
                .manifest
                .layout
                .as_ref()
                .and_then(|layout| layout.targets.as_deref())
                == Some("wasm32")
        })
        .map(|member| member.package_name)
        .collect()
}

/// Every member declaring the wasm32 target is linted, `browser_platform` among them.
#[test]
fn every_declared_wasm32_member_is_linted() {
    let declared = declared_wasm32_packages();
    assert!(
        declared.iter().any(|package| package == "browser_platform"),
        "{declared:?}"
    );
    let linted = wasm32_lint_packages(&root()).expect("the lint packages derive");
    for package in &declared {
        assert!(
            linted.contains(package),
            "{package} is not linted: {linted:?}"
        );
    }
}

/// Every crate of the frontend family under `crates/frontend` is linted too, the single-page app
/// and the offline service worker among them, and nothing else is.
#[test]
fn the_lint_covers_exactly_the_wasm32_members_and_the_frontend_crates() {
    let members = read_workspace_members(&root()).expect("the workspace members read");
    let mut expected = declared_wasm32_packages();
    for member in &members {
        if member.path.starts_with("crates/frontend/") && !expected.contains(&member.package_name) {
            expected.push(member.package_name.clone());
        }
    }
    for application in ["frontend_application", "offline_service_worker"] {
        assert!(
            expected.iter().any(|package| package == application),
            "{application} is no crate under crates/frontend: {expected:?}"
        );
    }
    let mut linted = wasm32_lint_packages(&root()).expect("the lint packages derive");
    expected.sort();
    linted.sort();
    assert_eq!(linted, expected);
}

/// The two lanes together lint the whole set, each package once: `wasm-ci` everything but the
/// frontend family, and `ci-local-leptos` the family with every target, through its derived
/// wasm32 line.
#[test]
fn wasm_ci_and_the_frontend_lane_partition_the_lint() {
    use crate::frontend_package_lane::{
        FRONTEND_LANE, FrontendLine, frontend_line_argv, frontend_line_of, frontend_packages,
    };
    let all = wasm32_lint_packages(&root()).expect("the lint packages derive");
    let wasm_ci = wasm_ci_lint_packages(&root()).expect("the wasm-ci packages derive");
    let family = frontend_packages(&root()).expect("the frontend family derives");
    for package in &family {
        assert!(!wasm_ci.contains(package), "{package} twice");
    }
    let row = crate::task_runner::find(FRONTEND_LANE).expect("the frontend lane is a task row");
    let lints_for_wasm32 = row.steps.iter().any(|step| {
        matches!(step, crate::task_runner::Step::Native { run }
            if frontend_line_of(*run) == Some(FrontendLine::Wasm32Clippy))
    });
    assert!(
        lints_for_wasm32,
        "`{FRONTEND_LANE}` has no wasm32 lint step"
    );
    let line = frontend_line_argv(&root(), FrontendLine::Wasm32Clippy)
        .expect("the frontend line derives")
        .join(" ");
    for package in &family {
        assert!(
            format!("{line} ").contains(&format!("-p {package} ")),
            "`{FRONTEND_LANE}` does not lint {package} for wasm32: {line}"
        );
    }
    let mut union: Vec<String> = wasm_ci.into_iter().chain(family).collect();
    let mut all_sorted = all;
    union.sort();
    all_sorted.sort();
    assert_eq!(union, all_sorted);
}

/// A `targets = "any"` crate under `crates/frontend` is linted for wasm32, by the frontend lane
/// and not by `wasm-ci`, the two shell crates beside it; a `targets = "any"` crate elsewhere is
/// not linted for wasm32, and a declared wasm32 crate elsewhere is linted by `wasm-ci`.
#[test]
fn an_any_target_frontend_crate_is_linted_for_wasm32_by_the_frontend_lane() {
    let folder = std::env::temp_dir().join(format!(
        "wasm32-lint-lane-any-frontend-crate-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    let write = |relative: &str, body: &str| {
        let path = folder.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).unwrap();
        std::fs::write(path, body).unwrap();
    };
    let any = "\n[package.metadata.layout]\ntargets = \"any\"\n";
    let wasm32 = "\n[package.metadata.layout]\ntargets = \"wasm32\"\n";
    write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/foundation/*\", \"crates/frontend/*/*\"]\n",
    );
    write(
        "crates/frontend/shell/frontend_application/Cargo.toml",
        "[package]\nname = \"frontend_application\"\n",
    );
    write(
        "crates/frontend/shell/offline_service_worker/Cargo.toml",
        &format!("[package]\nname = \"offline_service_worker\"\n{any}"),
    );
    write(
        "crates/foundation/browser_platform/Cargo.toml",
        &format!("[package]\nname = \"browser_platform\"\n{wasm32}"),
    );
    write(
        "crates/foundation/time_source/Cargo.toml",
        &format!("[package]\nname = \"time_source\"\n{any}"),
    );
    write(
        "crates/frontend/foundation/frontend_ui/Cargo.toml",
        &format!("[package]\nname = \"frontend_ui\"\n{any}"),
    );
    let mut all = wasm32_lint_packages(&folder).expect("the fixture reads");
    let wasm_ci = wasm_ci_lint_packages(&folder).expect("the fixture reads");
    std::fs::remove_dir_all(&folder).unwrap();
    all.sort();
    assert_eq!(
        all,
        [
            "browser_platform",
            "frontend_application",
            "frontend_ui",
            "offline_service_worker"
        ]
    );
    assert_eq!(wasm_ci, ["browser_platform"]);
}

/// The command line names each package once, then the wasm32 target and `-D warnings`.
#[test]
fn the_clippy_argv_names_each_package_and_the_target() {
    let argv = wasm32_clippy_argv(&["a".to_string(), "b".to_string()]);
    assert_eq!(
        argv.join(" "),
        "cargo clippy -p a -p b --target wasm32-unknown-unknown -- -D warnings"
    );
}

/// A workspace without the single-page app is refused, never a smaller lint: the offline service
/// worker beside it does not stand in for it.
#[test]
fn a_workspace_without_the_frontend_application_is_refused() {
    let folder = std::env::temp_dir().join(format!(
        "wasm32-lint-lane-missing-application-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("crates/frontend/shell/offline_service_worker")).unwrap();
    std::fs::write(
        folder.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/frontend/shell/offline_service_worker\"]\n",
    )
    .unwrap();
    std::fs::write(
        folder.join("crates/frontend/shell/offline_service_worker/Cargo.toml"),
        "[package]\nname = \"offline_service_worker\"\n",
    )
    .unwrap();
    let refused = wasm32_lint_packages(&folder).expect_err("a missing application is refused");
    let refused_wasm_ci =
        wasm_ci_lint_packages(&folder).expect_err("a missing application is refused");
    std::fs::remove_dir_all(&folder).unwrap();
    for error in [refused, refused_wasm_ci] {
        assert!(
            error.to_string().contains("`frontend_application`"),
            "{error}"
        );
    }
}
