//! Platform side effects for ticket execution and cleanup.
use anyhow::Result;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

pub fn cmd_clean(root: &Path, registry: &Value, id: &str) -> Result<()> {
    let targets = ticket_engine::cli::cleanup_targets(root, registry, id)?;
    let wt = targets.worktree;
    let branch = targets.branch;
    if wt.is_dir() {
        let status = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&wt)
            .current_dir(root)
            .status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            let _ = fs::remove_dir_all(&wt);
        }
        println!("Removed worktree {}", wt.display());
    }
    let check = Command::new("git")
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{branch}"))
        .current_dir(root)
        .status()?;
    if check.success() {
        Command::new("git")
            .args(["branch", "-D"])
            .arg(&branch)
            .current_dir(root)
            .status()?;
        println!("Deleted local branch {branch}");
    }
    Ok(())
}

pub fn cmd_done(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    cmd_clean(root, registry, id)?;
    ticket_engine::cli::cmd_ship(root, registry, id)
}

pub fn cmd_run(root: &Path, registry: &Value, dry_run: bool, stream: Option<&str>) -> Result<()> {
    ticket_engine::cli::cmd_run(root, registry, dry_run, stream, |root, registry, id| {
        crate::commands::platform::slice_execution::run_slice(
            root,
            registry,
            id,
            &crate::commands::platform::slice_execution::SliceRunOpts::default(),
        )
        .map(|_| ())
    })
}

#[cfg(test)]
#[path = "tests/execution_tests.rs"]
mod tests;
