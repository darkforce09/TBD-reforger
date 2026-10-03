//! Unit tests for [`crate::api_package_lane`]: the rendered API lines and their derivation from
//! this checkout's workspace.

use super::*;

fn packages() -> Vec<String> {
    ["api", "api_database", "api_state"]
        .map(String::from)
        .to_vec()
}

#[test]
fn each_line_names_every_package_then_its_own_words() {
    let rendered = |line| api_cargo_argv(line, &packages()).join(" ");
    assert_eq!(
        rendered(ApiLine::Test),
        "cargo test -p api -p api_database -p api_state"
    );
    assert_eq!(
        rendered(ApiLine::UnitTests),
        "cargo test -p api -p api_database -p api_state --lib --bins"
    );
    assert_eq!(
        rendered(ApiLine::Clippy),
        "cargo clippy -p api -p api_database -p api_state --all-targets -- -D warnings"
    );
    assert_eq!(
        rendered(ApiLine::Build),
        "cargo build -p api -p api_database -p api_state --all-targets"
    );
}

/// This checkout's lines name `api` and every API crate on disk.
#[test]
fn this_checkout_lines_name_every_api_crate() {
    let root = tool_test_support::test_repo_root();
    let argv = api_line_argv(&root, ApiLine::Clippy).expect("the API packages derive");
    let named: Vec<&str> = argv
        .windows(2)
        .filter(|pair| pair[0] == "-p")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(named.first(), Some(&"api"), "{argv:?}");
    for crate_folder in std::fs::read_dir(root.join("crates/api"))
        .expect("crates/api exists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
    {
        let name = crate_folder.file_name();
        let name = name.to_string_lossy();
        assert!(
            named.contains(&name.as_ref()),
            "{name} is not linted: {argv:?}"
        );
    }
}

#[test]
fn an_unreadable_workspace_is_an_error_not_an_api_only_line() {
    let missing = std::env::temp_dir().join(format!("api-lane-missing-{}", std::process::id()));
    let error = api_line_argv(&missing, ApiLine::Build).expect_err("no workspace");
    assert!(error.to_string().contains("API packages"), "{error}");
}

#[test]
fn each_runner_maps_to_its_line_and_nothing_else_does() {
    assert_eq!(api_line_of(run_api_test), Some(ApiLine::Test));
    assert_eq!(api_line_of(run_api_unit_tests), Some(ApiLine::UnitTests));
    assert_eq!(api_line_of(run_api_clippy), Some(ApiLine::Clippy));
    assert_eq!(api_line_of(run_api_build), Some(ApiLine::Build));
    assert_eq!(api_line_of(crate::wasm32_lint_lane::run_wasm_ci_lint), None);
}
