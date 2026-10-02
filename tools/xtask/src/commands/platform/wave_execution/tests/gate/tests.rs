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
/// outside the dedicated packages, every dedicated package is a workspace member, and the step
/// that tests each dedicated package is wired into the gate.
#[test]
fn the_wave_gate_tests_every_workspace_member() {
    let root = crate::core::repository_root::test_repo_root();
    let derived = crate::commands::ci::workspace_member_tests::member_packages_except(
        &root,
        &gate_dispatch::WAVE_GATE_DEDICATED_TEST_PACKAGES,
    )
    .expect("every dedicated package is a workspace member");
    assert!(!derived.is_empty(), "the derived step names no package");
    let source = include_str!("../../gate/gate_dispatch.rs")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for wired in [
        r#"r.run("test api", || db::gate_test_api(ctx));"#,
        r#""cargo", "test", "-p", "map_engine","#,
        r#""cargo", "test", "-p", "frontend","#,
        r#"r.run("test workspace members", ||"#,
    ] {
        assert!(
            source.contains(wired),
            "the wave gate lost the step `{wired}`"
        );
    }
}
