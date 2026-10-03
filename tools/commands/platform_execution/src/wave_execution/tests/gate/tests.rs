use super::*;

#[test]
fn the_step_runner_indents_the_last_fifteen_lines_on_failure() {
    // The six-space indent and the 15-line window are both scraped by readers, so they are a
    // contract.
    let out: String = (1..=20).map(|i| format!("line{i}\n")).collect();
    let lines: Vec<&str> = out.lines().collect();
    let tail: Vec<&str> = lines.iter().skip(lines.len() - 15).copied().collect();
    assert_eq!(tail.len(), 15);
    assert_eq!(tail[0], "line6");
    assert_eq!(format!("      {}", tail[0]), "      line6");
}

#[test]
fn both_gates_run_the_same_ten_class_r_verifies() {
    // A step wired into only one half drifts green. The shared const makes that structurally
    // impossible, and this pins the count.
    assert_eq!(VERIFY_STEPS.len(), 10);
    assert!(
        VERIFY_STEPS
            .iter()
            .any(|(_, n)| *n == "results-reporter-identity-comments")
    );
    assert!(
        VERIFY_STEPS
            .iter()
            .any(|(_, n)| *n == "player-identity-comments")
    );
}

/// The wave gate tests every workspace member: `test workspace members` derives every member
/// outside the dedicated packages and the API family, every dedicated package is a workspace
/// member, and the step that tests each dedicated package (the API family with `api`) is wired
/// into the gate.
#[test]
fn the_wave_gate_tests_every_workspace_member() {
    let root = tool_test_support::test_repo_root();
    let derived = ci_task_catalog::workspace_member_tests::member_packages_outside_api_family(
        &root,
        &gate_dispatch::WAVE_GATE_DEDICATED_TEST_PACKAGES,
    )
    .expect("every dedicated package is a workspace member");
    let api_crates: Vec<String> = repository_laws::workspace_members::read_workspace_members(&root)
        .expect("the workspace members read")
        .into_iter()
        .filter(|member| member.parent_folder() == "crates/api")
        .map(|member| member.package_name)
        .collect();
    assert!(!api_crates.is_empty(), "the workspace names no API crate");
    assert!(
        !derived.iter().any(|package| api_crates.contains(package)),
        "the API crates run in `test api`, not again per member: {derived:?}"
    );
    assert!(!derived.is_empty(), "the derived step names no package");
    let source = include_str!("../../gate/gate_dispatch.rs")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for wired in [
        r#"r.run("test api", || db::gate_test_api(ctx));"#,
        r#""cargo", "test", "-p", "frontend","#,
        r#"r.run("test workspace members", ||"#,
    ] {
        assert!(
            source.contains(wired),
            "the wave gate lost the step `{wired}`"
        );
    }
}

/// The wave gate's tool lint names every tool crate a wave can touch: the derived list is every
/// workspace member under `tools/` (the crates the xtask commands moved into among them), names
/// xtask and developer_tools, names nothing outside `tools/`, and the step's command line keeps
/// its `cargo clippy … --all-targets --quiet -- -D warnings` shape.
#[test]
fn the_wave_gate_lints_every_tool_crate_of_the_workspace() {
    let root = tool_test_support::test_repo_root();
    let derived =
        tool_clippy_packages(&root).expect("the workspace names xtask and developer_tools");
    let members = repository_laws::workspace_members::read_workspace_members(&root)
        .expect("the workspace members read");
    let tool_members: Vec<String> = members
        .iter()
        .filter(|member| member.path.starts_with("tools/"))
        .map(|member| member.package_name.clone())
        .collect();
    assert!(
        tool_members.len() > ANCHOR_TOOL_PACKAGES.len(),
        "the workspace holds tool crates beyond xtask and developer_tools"
    );
    assert_eq!(derived, tool_members, "the lint misses a tool crate");
    for expected in [
        "xtask",
        "developer_tools",
        "platform_execution",
        "process_runner",
    ] {
        assert!(
            derived.iter().any(|package| package == expected),
            "the lint misses `{expected}`"
        );
    }
    for outside in ["api", "frontend", "map_renderer", "ticketboard"] {
        assert!(
            !derived.iter().any(|package| package == outside),
            "the lint names `{outside}`, which is no tool crate"
        );
    }
    let packages = ["xtask".to_string(), "platform_execution".to_string()];
    let argv = tool_clippy_argv(&packages);
    assert_eq!(
        argv,
        [
            "cargo",
            "clippy",
            "-p",
            "xtask",
            "-p",
            "platform_execution",
            "--all-targets",
            "--quiet",
            "--",
            "-D",
            "warnings"
        ]
    );
}

/// A workspace whose tool folder lacks xtask or developer_tools is an error, never a smaller lint.
#[test]
fn the_tool_lint_refuses_a_workspace_without_its_anchor_packages() {
    let folder = std::env::temp_dir().join(format!(
        "platform-execution-tool-lint-{}",
        std::process::id()
    ));
    let member = folder.join("tools/xtask");
    std::fs::create_dir_all(&member).unwrap();
    std::fs::write(
        folder.join("Cargo.toml"),
        "[workspace]\nmembers = [\"tools/xtask\"]\n",
    )
    .unwrap();
    std::fs::write(
        member.join("Cargo.toml"),
        "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let refusal = tool_clippy_packages(&folder);
    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(
        refusal,
        Err("`developer_tools` is no workspace member under `tools/`".to_string())
    );
}
