//! Unit tests for [`crate::wasm32_lint_lane`]: the derived package set over this checkout, its
//! split between the two lanes that lint it, and the refusals over fixture workspaces.

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

/// The browser applications are linted too, and nothing else is.
#[test]
fn the_lint_covers_exactly_the_wasm32_members_and_the_applications() {
    let members: Vec<String> = read_workspace_members(&root())
        .expect("the workspace members read")
        .into_iter()
        .map(|member| member.package_name)
        .collect();
    let mut expected = declared_wasm32_packages();
    for package in &BROWSER_APPLICATIONS {
        if members.iter().any(|member| member == package) {
            expected.push((*package).to_string());
        }
    }
    let mut linted = wasm32_lint_packages(&root()).expect("the lint packages derive");
    expected.sort();
    linted.sort();
    assert_eq!(linted, expected);
}

/// The two lanes together lint the whole set, each package once: `wasm-ci` everything but the
/// frontend, and `ci-local-leptos` the frontend with every target.
#[test]
fn wasm_ci_and_the_own_lanes_partition_the_lint() {
    let all = wasm32_lint_packages(&root()).expect("the lint packages derive");
    let wasm_ci = wasm_ci_lint_packages(&root()).expect("the wasm-ci packages derive");
    for (package, lane) in OWN_LANE_WASM32_LINTS {
        assert!(!wasm_ci.iter().any(|p| p == package), "{package} twice");
        let row = crate::task_runner::find(lane).expect("the own lane is a task row");
        let lints_it = row.steps.iter().any(|step| {
            crate::task_runner::step_echo(step).is_some_and(|line| {
                line.starts_with("cargo clippy")
                    && line.contains(&format!("-p {package} "))
                    && line.contains("--target wasm32-unknown-unknown")
            })
        });
        assert!(lints_it, "`{lane}` does not lint {package} for wasm32");
    }
    let mut union: Vec<String> = wasm_ci
        .into_iter()
        .chain(OWN_LANE_WASM32_LINTS.iter().map(|(p, _)| (*p).to_string()))
        .collect();
    let mut all_sorted = all;
    union.sort();
    all_sorted.sort();
    assert_eq!(union, all_sorted);
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

/// A workspace without one of the browser applications is refused, never a smaller lint.
#[test]
fn a_missing_browser_application_is_refused() {
    let folder = std::env::temp_dir().join(format!(
        "wasm32-lint-lane-missing-application-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("apps/frontend")).unwrap();
    std::fs::write(
        folder.join("Cargo.toml"),
        "[workspace]\nmembers = [\"apps/frontend\"]\n",
    )
    .unwrap();
    std::fs::write(
        folder.join("apps/frontend/Cargo.toml"),
        "[package]\nname = \"frontend\"\n",
    )
    .unwrap();
    let refused = wasm32_lint_packages(&folder).expect_err("a missing application is refused");
    std::fs::remove_dir_all(&folder).unwrap();
    assert!(
        refused.to_string().contains("`offline_service_worker`"),
        "{refused}"
    );
}
