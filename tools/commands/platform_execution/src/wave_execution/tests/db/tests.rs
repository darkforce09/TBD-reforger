//! Unit tests for [`super::gate_test_api_argv`]: the `test api` step covers every API crate.

use super::*;

/// The step runs the derived API test line in the private gate folder, every `-p` intact.
#[test]
fn the_test_api_step_runs_the_derived_line_in_the_gate_folder() {
    let line: Vec<String> = ["cargo", "test", "-p", "api_server", "-p", "api_state"]
        .map(String::from)
        .to_vec();
    assert_eq!(
        gate_test_api_argv("/target/gate-api", &line).join(" "),
        "env CARGO_TARGET_DIR=/target/gate-api CARGO_INCREMENTAL=0 cargo test -p api_server \
         -p api_state --quiet -- --nocapture"
    );
}

/// This checkout's `test api` line names `api_server` first, every API crate on disk (`api_server`
/// among them) and the one member outside `crates/api` that uses an API crate
/// (`staging_fixtures`), each once.
#[test]
fn this_checkout_test_api_line_names_every_api_crate() {
    let root = tool_test_support::test_repo_root();
    let line = api_line_argv(&root, ApiLine::Test).expect("the API packages derive");
    let argv = gate_test_api_argv("/target/gate-api", &line);
    let named: Vec<&str> = argv
        .windows(2)
        .filter(|pair| pair[0] == "-p")
        .map(|pair| pair[1].as_str())
        .collect();
    let on_disk: Vec<String> = std::fs::read_dir(root.join("crates/api"))
        .expect("crates/api exists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(named.first(), Some(&"api_server"), "{argv:?}");
    assert!(
        on_disk.iter().any(|api_crate| api_crate == "api_server"),
        "{on_disk:?}"
    );
    for package in &named {
        assert_eq!(
            named.iter().filter(|other| *other == package).count(),
            1,
            "{package} is named twice: {argv:?}"
        );
    }
    for api_crate in &on_disk {
        assert!(
            named.contains(&api_crate.as_str()),
            "{api_crate} missing: {argv:?}"
        );
    }
    assert!(named.contains(&"staging_fixtures"), "{argv:?}");
    assert_eq!(named.len(), on_disk.len() + 1, "{argv:?}");
}
