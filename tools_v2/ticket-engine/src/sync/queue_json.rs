//! Queue json.

use super::*;

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
        "worktree_base": ".ai/artifacts/worktrees",
        "git_base": "main",
        "tickets": tickets_out,
    })
}
