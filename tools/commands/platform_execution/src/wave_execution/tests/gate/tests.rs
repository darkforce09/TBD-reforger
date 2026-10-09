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
/// member, and the step that tests each dedicated package (the API family with `api_server`) is
/// wired into the gate.
#[test]
fn the_wave_gate_tests_every_workspace_member() {
    let root = tool_test_support::test_repo_root();
    let dedicated = gate_dispatch::wave_gate_dedicated_test_packages(&root)
        .expect("the frontend family derives");
    let family = ci_task_catalog::frontend_package_lane::frontend_packages(&root)
        .expect("the frontend family derives");
    assert_eq!(dedicated[0], gate_dispatch::WAVE_GATE_API_TEST_PACKAGE);
    assert_eq!(
        dedicated[1..],
        family[..],
        "`test frontend` tests the whole family"
    );
    let dedicated: Vec<&str> = dedicated.iter().map(String::as_str).collect();
    let derived = ci_task_catalog::workspace_member_tests::member_packages_outside_api_family(
        &root, &dedicated,
    )
    .expect("every dedicated package is a workspace member");
    assert!(
        !derived.iter().any(|package| family.contains(package)),
        "the frontend family runs in `test frontend`, not again per member: {derived:?}"
    );
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
        r#"r.run("test frontend", || { frontend_family_step( ctx, hostrun, &["env", &frontend_dir, "cargo", "test"],"#,
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
        "ticketboard_desktop",
    ] {
        assert!(
            derived.iter().any(|package| package == expected),
            "the lint misses `{expected}`"
        );
    }
    for outside in [
        "api_server",
        "frontend_application",
        "map_renderer",
        "game_server_host_agent",
    ] {
        assert!(
            !derived.iter().any(|package| package == outside),
            "the lint names `{outside}`, which is no tool crate"
        );
    }
    let packages = ["xtask".to_string(), "platform_execution".to_string()];
    let argv = native_clippy_argv(&packages);
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

/// The wave gate's clippy lanes cover every workspace member: the tool lint, the frontend family,
/// the wasm32 members and the native lint together name each member of the root manifest; the
/// native lint names the API server, every API crate, the game server host agent and the
/// `crates/**` libraries outside the frontend family, and nothing another lane already lints; the
/// offline service worker is linted with the frontend family alone; and both derived steps are
/// wired into the gate.
#[test]
fn the_wave_gate_clippy_lanes_cover_every_workspace_member() {
    let root = tool_test_support::test_repo_root();
    let members = repository_laws::workspace_members::read_workspace_members(&root)
        .expect("the workspace members read");
    let tools = tool_clippy_packages(&root).expect("the tool lane derives");
    let frontend = ci_task_catalog::frontend_package_lane::frontend_packages(&root)
        .expect("the frontend family derives");
    let wasm32 =
        clippy_package_sets::wasm32_clippy_packages(&root).expect("the wasm32 lane derives");
    let native =
        clippy_package_sets::native_clippy_packages(&root).expect("the native lane derives");
    for member in &members {
        let package = &member.package_name;
        let lanes = [&tools, &frontend, &wasm32, &native]
            .iter()
            .filter(|lane| lane.contains(package))
            .count();
        assert_eq!(
            lanes, 1,
            "`{package}` ({}) is linted by {lanes} clippy lanes, not exactly one",
            member.path
        );
    }
    let api_family: Vec<&str> = members
        .iter()
        .filter(|member| member.parent_folder() == "crates/api")
        .map(|member| member.package_name.as_str())
        .collect();
    assert!(!api_family.is_empty(), "the workspace names no API crate");
    for package in api_family.iter().copied().chain([
        "api_server",
        "geometry_primitives",
        "game_server_host_agent",
    ]) {
        assert!(
            native.iter().any(|linted| linted == package),
            "the native lint misses `{package}`"
        );
    }
    assert!(
        native.iter().all(|package| members
            .iter()
            .any(|m| &m.package_name == package && m.path.starts_with("crates/"))),
        "the native lint names a member outside `crates/`: {native:?}"
    );
    assert!(
        frontend
            .iter()
            .any(|package| package == "offline_service_worker"),
        "the frontend family misses the offline service worker: {frontend:?}"
    );
    let source = include_str!("../../gate/gate_dispatch.rs")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for wired in [
        r#"r.run("clippy native crates", || { match native_clippy_packages(&ctx.root) { Ok(packages) => checkrun(ctx, &native_clippy_argv(&packages)),"#,
        r#"r.run("clippy wasm32 members", || { match wasm32_clippy_packages(&ctx.root) {"#,
        r#"let argv = wasm32_clippy_argv(&packages);"#,
    ] {
        assert!(
            source.contains(wired),
            "the wave gate lost the step `{wired}`"
        );
    }
}

/// A workspace whose native lane lacks the API server is an error, never a smaller lint.
#[test]
fn the_native_lint_refuses_a_workspace_without_the_api_server() {
    let folder = std::env::temp_dir().join(format!(
        "platform-execution-native-lint-{}",
        std::process::id()
    ));
    let members = [
        (
            "crates/frontend/shell/frontend_application",
            "frontend_application",
        ),
        (
            "crates/frontend/shell/offline_service_worker",
            "offline_service_worker",
        ),
        ("crates/geometry/geometry_primitives", "geometry_primitives"),
    ];
    for (path, package) in members {
        let member = folder.join(path);
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }
    let listed: Vec<String> = members
        .iter()
        .map(|(path, _)| format!("\"{path}\""))
        .collect();
    std::fs::write(
        folder.join("Cargo.toml"),
        format!("[workspace]\nmembers = [{}]\n", listed.join(", ")),
    )
    .unwrap();
    let refusal = clippy_package_sets::native_clippy_packages(&folder);
    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(
        refusal,
        Err(
            "`api_server` is no workspace member outside `tools/`, the frontend family and the \
             wasm32 members"
                .to_string()
        )
    );
}
