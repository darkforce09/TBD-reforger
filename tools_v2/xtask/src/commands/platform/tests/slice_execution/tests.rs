use super::*;
use serde_json::json;
use std::fs;

fn fixture_path(name: &str) -> PathBuf {
    crate::core::repository_root::test_repo_root()
        .join("tools_v2/ticket-engine/tests/fixtures/execution_receipts")
        .join(name)
}

fn scratch(tag: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("tbd-slice-run-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join(".ai/tickets")).expect("mk scratch");
    tmp
}

fn plant_registry(root: &Path, id: &str) -> Registry {
    fs::write(root.join("spec.md"), "# spec\n").unwrap();
    json!({
        "next_id": 1,
        "tickets": [{
            "id": id,
            "kind": "work",
            "title": "t",
            "summary": "t",
            "status": "ready",
            "order": 1,
            "spec": "spec.md",
            "executor": "claude-code",
            "main_goal": "as a tester I produce a run receipt",
            "acceptance": ["writes a receipt"],
            "scope": { "repo": { "layers": ["xtask"] } }
        }]
    })
}

/// A stub agent binary written at TEST RUNTIME (never committed): echoes a recorded
/// fixture, exactly like a real CLI printing its final JSON.
fn write_stub(dir: &Path, fixture: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let stub = dir.join("stub-agent");
    fs::write(
        &stub,
        format!("#!/bin/sh\ncat '{}'\n", fixture_path(fixture).display()),
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    stub
}

fn opts_with(fixture: Option<&str>, stub: Option<PathBuf>) -> SliceRunOpts {
    SliceRunOpts {
        fixture: fixture.map(fixture_path),
        started: Some("2026-08-14T01:00:00Z".to_string()),
        agent_cmd_override: stub.map(|s| vec![s.to_string_lossy().into_owned()]),
        dry_run: false,
    }
}

#[test]
fn stub_binary_run_writes_a_receipt_with_the_recorded_tokens() {
    let tmp = scratch("stub-ok");
    let reg = plant_registry(&tmp, "T-990");
    let stub = write_stub(&tmp, "slice_run_cursor_agent.json");
    let path = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub)))
        .unwrap()
        .expect("not a dry run");
    let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(rec.agent, "stub-agent", "agent = basename of the command");
    assert_eq!(rec.tokens_consumed.input, 12000);
    assert_eq!(rec.tokens_consumed.total, 23600);
    assert_eq!(rec.outcome.as_deref(), Some("ran"));
    // scratch root is not a git repo → no sha → `nosha` in the filename.
    assert!(
        path.to_string_lossy().contains("nosha"),
        "{}",
        path.display()
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn stub_binary_claude_dialect_parses_too() {
    let tmp = scratch("stub-claude");
    let reg = plant_registry(&tmp, "T-990");
    let stub = write_stub(&tmp, "slice_run_claude_print.json");
    let path = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub)))
        .unwrap()
        .expect("not a dry run");
    let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(rec.tokens_consumed.total, 36);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn exit_zero_without_usage_fails_and_writes_no_file() {
    let tmp = scratch("stub-no-usage");
    let reg = plant_registry(&tmp, "T-990");
    let stub = write_stub(&tmp, "slice_run_no_usage.json");
    let err = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub))).unwrap_err();
    assert!(format!("{err:#}").contains("run FAILED"), "{err:#}");
    assert!(
        !metrics::metrics_root(&tmp).join("T-990").exists(),
        "no receipt may exist after a failed run"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn fixture_replay_writes_a_receipt_without_spawning() {
    let tmp = scratch("fixture");
    let reg = plant_registry(&tmp, "T-990");
    let path = run_slice(
        &tmp,
        &reg,
        "T-990",
        &opts_with(Some("slice_run_claude_print.json"), None),
    )
    .unwrap()
    .expect("not a dry run");
    let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(rec.tokens_consumed.input, 10);
    assert_eq!(rec.tokens_consumed.total, 36);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn malformed_started_override_is_refused() {
    let tmp = scratch("bad-started");
    let reg = plant_registry(&tmp, "T-990");
    let mut opts = opts_with(Some("slice_run_claude_print.json"), None);
    opts.started = Some("yesterday".to_string());
    let err = run_slice(&tmp, &reg, "T-990", &opts).unwrap_err();
    assert!(format!("{err:#}").contains("RFC 3339"), "{err:#}");
    let _ = fs::remove_dir_all(&tmp);
}

/// Phase-2 tree shape: slice files are folded into the parent's `slice_plan`, not
/// top-level rows. A slice id must resolve through the plan; a parent id must
/// resolve to its ACTIVE slice — so the receipt directory is ALWAYS the slice id
/// that `platform wave land` later stamps.
#[test]
fn slice_ids_resolve_through_the_parent_slice_plan() {
    let tmp = scratch("slice-plan");
    fs::write(tmp.join("spec.md"), "# spec\n").unwrap();
    let reg = json!({
        "next_id": 1,
        "tickets": [{
            "id": "T-990",
            "kind": "program",
            "title": "t",
            "summary": "t",
            "status": "ready",
            "order": 1,
            "executor": "claude-code",
            "active_slice": "T-990.2",
            "slice_plan": {
                "T-990.2": {
                    "spec": "spec.md",
                    "executor": "claude-code",
                    "status": "ready",
                    "targets": ["root"]
                }
            }
        }]
    });
    // Direct slice id → plan entry.
    let path = run_slice(
        &tmp,
        &reg,
        "T-990.2",
        &opts_with(Some("slice_run_claude_print.json"), None),
    )
    .unwrap()
    .expect("not a dry run");
    assert!(
        path.to_string_lossy().contains("/T-990.2/"),
        "receipt under the SLICE id: {}",
        path.display()
    );
    // Parent id → active slice; SAME receipt directory.
    let path2 = run_slice(
        &tmp,
        &reg,
        "T-990",
        &opts_with(Some("slice_run_claude_print.json"), None),
    )
    .unwrap()
    .expect("not a dry run");
    assert!(
        path2.to_string_lossy().contains("/T-990.2/"),
        "parent id resolves to the active slice: {}",
        path2.display()
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn non_claude_code_executor_is_refused() {
    let tmp = scratch("executor");
    let mut reg = plant_registry(&tmp, "T-990");
    reg["tickets"][0]["executor"] = json!("workbench");
    let err = run_slice(
        &tmp,
        &reg,
        "T-990",
        &opts_with(Some("slice_run_claude_print.json"), None),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("executor is workbench"),
        "{err:#}"
    );
    let _ = fs::remove_dir_all(&tmp);
}
