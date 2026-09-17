use super::*;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ticket-cleanup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .arg(&root)
                .status()
                .unwrap()
                .success()
        );
        Self(root)
    }

    fn worktree(&self) -> PathBuf {
        let target = self.0.join(".ai/artifacts/worktrees/TBD-T-001");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("payload.txt"), "fixture").unwrap();
        target
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn cleanup_removes_an_unregistered_worktree_directory() {
    let fixture = Fixture::new();
    let target = fixture.worktree();
    let registry = json!({"tickets": [{"id": "T-001"}]});
    cmd_clean(&fixture.0, &registry, "T-001").unwrap();
    assert!(!target.exists());
    assert!(fixture.0.join(".git").is_dir());
}

#[test]
fn done_cleans_before_a_shipping_refusal() {
    let fixture = Fixture::new();
    let target = fixture.worktree();
    let mut registry = json!({"tickets": [{"id": "T-001"}]});
    assert!(cmd_done(&fixture.0, &mut registry, "T-001").is_err());
    assert!(!target.exists());
}
