//! Unit tests for [`crate::mod_wave`] (SIZE split — keep `mod_wave.rs` under 600).

use super::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn throwaway(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!(
        "/tmp/t853/w230/t890/ut-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::create_dir_all(root.join("docs/mod")).unwrap();
    fs::create_dir_all(root.join(".ai/artifacts/worktrees")).unwrap();
    fs::create_dir_all(root.join("scripts/mod")).unwrap();
    root
}

fn write_registry(root: &Path, slice_plan: &str) {
    let body = format!(
        "{open}{slice_plan}{close}",
        open = r#"{"tickets":[{"id":"T-181","slice_plan":{"#,
        close = r#"}}]}"#,
    );
    fs::write(root.join(".ai/tickets/registry.json"), body).unwrap();
}

#[test]
fn parent_slice_normalises_sub_slices() {
    assert_eq!(parent_slice("T-181.9.2"), "T-181.9");
    assert_eq!(parent_slice("T-181.9"), "T-181.9");
    assert_eq!(parent_slice("T-890"), "T-890");
}

#[test]
fn unknown_command_prints_help_rc2() {
    let root = throwaway("unknown");
    write_registry(&root, "");
    fs::write(root.join("docs/mod/wave_plan.tsv"), "").unwrap();
    let code = run_with_root(&root, &["nosuch".into()]);
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn status_absent_worktrees_on_scratch() {
    let root = throwaway("status");
    write_registry(&root, "");
    fs::write(
        root.join("docs/mod/wave_plan.tsv"),
        "99\tT-181.99\tFake slice for RED\towns\n99\tT-181.100\tFake two\towns\n",
    )
    .unwrap();
    // git repo so has_work/show-ref are quiet
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status();
    let code = run_with_root(&root, &["status".into()]);
    assert_eq!(code, 0);
    assert_eq!(current_wave(&root), "99");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn land_refuses_dirty_worktree() {
    let root = throwaway("land-dirty");
    write_registry(&root, "");
    fs::write(
        root.join("docs/mod/wave_plan.tsv"),
        "99\tT-181.99\tFake slice for RED\towns\n",
    )
    .unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status();
    let wt = root.join(BASE).join("T-181.99");
    fs::create_dir_all(&wt).unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&wt)
        .status();
    fs::write(wt.join("f"), "x\n").unwrap();
    let _ = Command::new("git")
        .current_dir(&wt)
        .args(["add", "f"])
        .status();
    let _ = Command::new("git")
        .current_dir(&wt)
        .args(["commit", "-qm", "x"])
        .status();
    fs::write(wt.join("f"), "dirty\n").unwrap();
    let code = run_with_root(&root, &["land".into()]);
    assert_eq!(code, 1, "bash went red on dirty land");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn prep_done_prints_nothing() {
    let root = throwaway("prep-done");
    // empty plan → current_wave == done
    write_registry(&root, r#""T-181.0":{"status":"shipped"}"#);
    fs::write(root.join("docs/mod/wave_plan.tsv"), "# only comments\n").unwrap();
    let code = run_with_root(&root, &["prep".into()]);
    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(&root);
}
