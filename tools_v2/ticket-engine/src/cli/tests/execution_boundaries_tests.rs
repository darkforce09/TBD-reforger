use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ticket-execution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("spec.md"), "# Ready ticket\n").unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn executable_registry() -> Value {
    json!({"tickets": [
        {"id": "T-001", "status": "ready", "order": 1, "spec": "spec.md", "executor": "claude-code"},
        {"id": "T-002", "status": "ready", "order": 2, "spec": "spec.md", "executor": "claude-code"}
    ]})
}

#[test]
fn batch_calls_executor_in_queue_order() {
    let fixture = Fixture::new();
    let registry = executable_registry();
    let mut calls = Vec::new();
    cmd_run(&fixture.0, &registry, false, None, |root, received, id| {
        assert_eq!(root, fixture.0);
        assert_eq!(received, &registry);
        calls.push(id.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(calls, ["T-001", "T-002"]);
}

#[test]
fn dry_run_does_not_call_executor() {
    let fixture = Fixture::new();
    cmd_run(&fixture.0, &executable_registry(), true, None, |_, _, _| {
        panic!("dry run must not execute an agent")
    })
    .unwrap();
}

#[test]
fn executor_failure_stops_the_batch() {
    let fixture = Fixture::new();
    let mut calls = Vec::new();
    let error = cmd_run(
        &fixture.0,
        &executable_registry(),
        false,
        None,
        |_, _, id| {
            calls.push(id.to_string());
            anyhow::bail!("executor refused")
        },
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "executor refused");
    assert_eq!(calls, ["T-001"]);
}

#[test]
fn cleanup_resolution_preserves_defaults_and_performs_no_deletion() {
    let fixture = Fixture::new();
    let target = fixture
        .0
        .join(crate::repository::WORKTREES_DIR)
        .join("TBD-T-001");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep.txt"), "retained").unwrap();
    let resolved = cleanup_targets(&fixture.0, &executable_registry(), "T-001").unwrap();
    assert_eq!(resolved.worktree, target);
    assert_eq!(resolved.branch, "ticket/T-001");
    assert_eq!(
        fs::read_to_string(target.join("keep.txt")).unwrap(),
        "retained"
    );
}

#[test]
fn cleanup_resolution_preserves_absolute_base_and_explicit_branch() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join(crate::repository::TICKETS_DIR)).unwrap();
    let base = fixture.0.join("custom-worktrees");
    fs::write(
        fixture.0.join(crate::repository::QUEUE_JSON),
        serde_json::to_vec(&json!({"worktree_base": base})).unwrap(),
    )
    .unwrap();
    let registry = json!({"tickets": [{"id": "T-001", "branch": "custom-ticket"}]});
    let resolved = cleanup_targets(&fixture.0, &registry, "T-001").unwrap();
    assert_eq!(resolved.worktree, base.join("TBD-T-001"));
    assert_eq!(resolved.branch, "custom-ticket");
}
