//! The dispatch queue, `queue.json`.
//!
//! **Role:** builds the `queue.json` value: the run defaults and every dispatchable ticket.
//! **Position:** under `ticket sync`; [`crate::sync::cmd_sync`] writes it and `set-status`
//! regenerates it alone.
//! **Signals & state:** none; a pure function of the registry value.
//! **Invariants:** only tickets in `ready`, `running` or `review` with a non-empty spec are
//! listed, in [`crate::registry::ticket_sort_key`] order.

use super::*;

/// The `queue.json` value of `registry`: `_comment`, `batch_size` 10, `concurrency` 3,
/// `worktree_base`, `git_base` `main`, and one `{id, title, status, spec, branch}` entry per
/// dispatchable ticket, where `spec` prefers the active slice's spec and `branch` defaults to
/// `ticket/<id>`.
pub fn generate_queue_json(registry: &Value) -> Value {
    let mut pipeline: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| {
            matches!(opt_str(t, "status"), Some("ready" | "running" | "review"))
                && !opt_str(t, "spec").unwrap_or("").trim().is_empty()
        })
        .collect();
    pipeline.sort_by_key(|t| ticket_sort_key(t));
    let tickets_out: Vec<Value> = pipeline
        .into_iter()
        .map(|t| {
            let tid = str_field(t, "id");
            let spec = {
                let s = slice_spec(t);
                if s.is_empty() {
                    opt_str(t, "spec").unwrap_or("").to_string()
                } else {
                    s
                }
            };
            let branch = opt_str(t, "branch")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("ticket/{tid}"));
            json!({
                "id": tid,
                "title": opt_str(t, "title").unwrap_or(""),
                "status": opt_str(t, "status").unwrap_or(""),
                "spec": spec,
                "branch": branch,
            })
        })
        .collect();
    let comment = AUTO_HEADER.trim();
    json!({
        "_comment": comment,
        "batch_size": 10,
        "concurrency": 3,
        "worktree_base": repository_layout::WORKTREES_DIR,
        "git_base": "main",
        "tickets": tickets_out,
    })
}
