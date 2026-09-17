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
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    crate::tickets_store::save_toml_tree(root, &v).unwrap();
}

/// A stub `.ai/tickets/wave.lock` — the T-912.2 successor to the stub TSVs these tests wrote.
fn write_lock(root: &Path, waves: &[(u32, &[&str])]) {
    let mut text = String::from("version = 1\nmax_concurrent = 8\npack_last = []\n");
    if waves.is_empty() {
        // toml emits an empty array-of-tables as an inline empty array; mirror it.
        text.push_str("waves = []\n");
    }
    for (n, ids) in waves {
        let ids = ids
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        text.push_str(&format!("\n[[waves]]\nn = {n}\ntickets = [{ids}]\n"));
    }
    text.push_str("\n[owns]\n\n[depends_on]\n");
    fs::write(root.join(".ai/tickets/wave.lock"), text).unwrap();
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
    write_lock(&root, &[]);
    let code = run_with_root(&root, &["nosuch".into()]);
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn status_absent_worktrees_on_scratch() {
    let root = throwaway("status");
    write_registry(&root, "");
    write_lock(&root, &[(99, &["T-181.99", "T-181.100"])]);
    // git repo so has_work/show-ref are quiet
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status();
    let code = run_with_root(&root, &["status".into()]);
    assert_eq!(code, 0);
    assert_eq!(current_wave(&root), Some("99".into()));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn land_refuses_dirty_worktree() {
    let root = throwaway("land-dirty");
    write_registry(&root, "");
    write_lock(&root, &[(99, &["T-181.99"])]);
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
    // no mod rows in the lock → current_wave == done
    write_registry(&root, r#""T-181.0":{"status":"shipped"}"#);
    write_lock(&root, &[]);
    let code = run_with_root(&root, &["prep".into()]);
    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_lock_is_a_refusal_not_all_shipped() {
    // The TSV-era shrug: missing plan → "ALL PLANNED WAVES SHIPPED", rc 0. Killed by T-912.2.
    let root = throwaway("missing-lock");
    write_registry(&root, "");
    assert_eq!(current_wave(&root), None);
    assert_eq!(run_with_root(&root, &["status".into()]), 2);
    assert_eq!(run_with_root(&root, &["land".into()]), 2);
    let _ = fs::remove_dir_all(&root);
}
