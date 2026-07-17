use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use crate::constants::*;
use crate::gap::sync_gap_analysis_ticket_column;
use crate::registry::*;
use crate::root::gap_analysis_path;

pub fn cmd_sync(root: &Path, registry: &Value) -> Result<()> {
    fs::create_dir_all(root.join("docs"))?;

    fs::write(
        root.join("docs/TICKET_REGISTRY.md"),
        generate_ticket_registry_md(registry),
    )?;
    fs::write(
        root.join("docs/TICKET_LEAD.md"),
        generate_ticket_lead_md(registry),
    )?;
    fs::write(
        root.join("docs/TICKET_DEV_QUEUE.md"),
        generate_ticket_dev_queue_md(registry),
    )?;
    fs::write(
        root.join("docs/TICKET_BRAINSTORM.md"),
        generate_ticket_brainstorm_md(registry),
    )?;
    fs::write(
        root.join("docs/TICKET_MOD_QUEUE.md"),
        generate_ticket_mod_queue_md(registry),
    )?;
    fs::write(
        root.join("docs/MILESTONES.md"),
        generate_milestones_md(registry),
    )?;

    let queue = generate_queue_json(registry);
    write_json_ascii(&root.join(".ai/tickets/queue.json"), &queue)?;

    let claude = root.join("CLAUDE.md");
    if claude.is_file() {
        let text = fs::read_to_string(&claude)?;
        if text.contains(STATUS_MARKER_START) {
            inject_status_block(root, registry)?;
        }
    }
    let roadmap = root.join("docs/specs/Mission_Creator_Architecture/ROADMAP.md");
    if roadmap.is_file() {
        let text = fs::read_to_string(&roadmap)?;
        if text.contains(NEXT_MARKER_START) {
            inject_next_block(root, registry)?;
        }
    }

    if gap_analysis_path(root).is_file() {
        sync_gap_analysis_ticket_column(root, registry)?;
    }

    println!("sync complete");
    Ok(())
}

fn em_dash_or_order(t: &Value) -> String {
    match t.get("order") {
        Some(v) if !matches!(v, Value::Null) => {
            if let Some(n) = v.as_i64() {
                n.to_string()
            } else if let Some(s) = v.as_str() {
                s.to_string()
            } else {
                v.to_string()
            }
        }
        _ => "—".to_string(),
    }
}

fn generate_ticket_registry_md(registry: &Value) -> String {
    let mut rows: Vec<&Value> = tickets(registry).iter().collect();
    rows.sort_by_key(|t| ticket_sort_key(t));
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Ticket Registry".into(),
        "".into(),
        "| T-ID | Order | Status | Program | Title | Summary |".into(),
        "|------|-------|--------|---------|-------|---------|".into(),
    ];
    for t in rows {
        let summary = str_field(t, "summary").replace('|', "\\|");
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            str_field(t, "id"),
            em_dash_or_order(t),
            str_field(t, "status"),
            opt_str(t, "program").unwrap_or(""),
            opt_str(t, "title").unwrap_or(""),
            summary,
        ));
    }
    lines.push("".into());
    lines.join("\n")
}

fn generate_ticket_lead_md(registry: &Value) -> String {
    let all = tickets(registry);
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Ticket Lead Dashboard".into(),
        "".into(),
    ];
    let sections: &[(&str, &[&str])] = &[
        ("Running / Review", &["running", "review", "active"]),
        ("Ready", &["ready"]),
        ("Next queued (top 10)", &["queued"]),
    ];
    for (label, statuses) in sections {
        let mut subset: Vec<&Value> = all
            .iter()
            .filter(|t| {
                opt_str(t, "status")
                    .map(|s| statuses.contains(&s))
                    .unwrap_or(false)
            })
            .collect();
        subset.sort_by_key(|t| ticket_sort_key(t));
        if label.starts_with("Next") {
            subset.truncate(10);
        }
        lines.push(format!("## {label}"));
        lines.push("".into());
        for t in subset {
            lines.push(format!(
                "- **{}** ({}) — {} [{}] — {}",
                str_field(t, "id"),
                em_dash_or_order(t),
                opt_str(t, "title").unwrap_or(""),
                opt_str(t, "status").unwrap_or(""),
                opt_str(t, "summary").unwrap_or(""),
            ));
        }
        lines.push("".into());
    }
    lines.push("## Dependency graph (scoped)".into());
    lines.push("".into());
    lines.push("```mermaid".into());
    lines.push("flowchart LR".into());

    let mut graph_ids: HashSet<String> = HashSet::new();
    for t in all {
        if matches!(opt_str(t, "status"), Some("running" | "review" | "ready")) {
            graph_ids.insert(str_field(t, "id"));
        }
    }
    let mut queued: Vec<&Value> = all
        .iter()
        .filter(|t| opt_str(t, "status") == Some("queued") && order_truthy(t))
        .collect();
    queued.sort_by_key(|t| ticket_sort_key(t));
    for t in queued.into_iter().take(5) {
        graph_ids.insert(str_field(t, "id"));
    }
    for t in all {
        if opt_str(t, "status") == Some("shipped") {
            let tid = str_field(t, "id");
            for other in all {
                if graph_ids.contains(&str_field(other, "id")) {
                    if let Some(deps) = other.get("depends_on").and_then(|d| d.as_array()) {
                        if deps.iter().any(|d| d.as_str() == Some(tid.as_str())) {
                            graph_ids.insert(tid.clone());
                        }
                    }
                }
            }
        }
    }
    for t in all {
        let tid = str_field(t, "id");
        if !graph_ids.contains(&tid) {
            continue;
        }
        if let Some(deps) = t.get("depends_on").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_s) = dep.as_str() {
                    if graph_ids.contains(dep_s) {
                        let dep_n = dep_s.replace('-', "");
                        let tid_n = tid.replace('-', "");
                        lines.push(format!("  {dep_n}[{dep_s}] --> {tid_n}[{tid}]"));
                    }
                }
            }
        }
    }
    lines.push("```".into());
    lines.push("".into());
    lines.join("\n")
}

fn generate_ticket_dev_queue_md(registry: &Value) -> String {
    let mut ready: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| {
            matches!(opt_str(t, "status"), Some("ready" | "active"))
                && slice_executor(t) == "claude-code"
        })
        .collect();
    ready.sort_by_key(|t| ticket_sort_key(t));
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Developer Queue".into(),
        "".into(),
        "Only `ready` tickets with `executor: claude-code` (or active slice).".into(),
        "".into(),
    ];
    for t in ready {
        let tid = str_field(t, "id");
        let branch = opt_str(t, "branch")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("ticket/{tid}"));
        let active = opt_str(t, "active_slice").unwrap_or("");
        lines.push(format!("## {tid} — {}", opt_str(t, "title").unwrap_or("")));
        lines.push("".into());
        if !active.is_empty() {
            lines.push(format!("- **Active slice:** `{active}`"));
        }
        lines.push(format!("- **Slice spec:** `{}`", slice_spec(t)));
        lines.push(format!(
            "- **Program hub:** `{}`",
            opt_str(t, "spec").unwrap_or("")
        ));
        lines.push(format!("- **Branch:** `{branch}`"));
        let targets = string_list(t, "targets").unwrap_or_default();
        lines.push(format!("- **Targets:** {}", targets.join(", ")));
        lines.push(format!(
            "- **Summary:** {}",
            opt_str(t, "summary").unwrap_or("")
        ));
        lines.push("".into());
    }
    lines.join("\n")
}

fn generate_ticket_mod_queue_md(registry: &Value) -> String {
    let mut rows: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| {
            matches!(
                opt_str(t, "status"),
                Some("ready" | "queued" | "running" | "review")
            ) && matches!(
                slice_executor(t).as_str(),
                "workbench" | "human"
            ) && string_list(t, "targets")
                .unwrap_or_default()
                .iter()
                .any(|x| x == "mod")
        })
        .collect();
    rows.sort_by_key(|t| ticket_sort_key(t));
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Mod / Workbench Queue".into(),
        "".into(),
        "Tickets for Workbench or human execution (`apps/mod/` targets).".into(),
        "".into(),
    ];
    for t in rows {
        lines.push(format!(
            "- **{}** ({}) — {} [{}] — milestone {}",
            str_field(t, "id"),
            opt_str(t, "status").unwrap_or(""),
            opt_str(t, "title").unwrap_or(""),
            slice_executor(t),
            opt_str(t, "milestone").unwrap_or("—"),
        ));
    }
    lines.push("".into());
    lines.join("\n")
}

fn generate_milestones_md(registry: &Value) -> String {
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Milestones (generated from tickets)".into(),
        "".into(),
        "Scheduling detail: [`docs/mod/MILESTONES.md`](mod/MILESTONES.md).".into(),
        "".into(),
    ];
    for milestone in ["M1", "M2"] {
        let mut subset: Vec<&Value> = tickets(registry)
            .iter()
            .filter(|t| opt_str(t, "milestone") == Some(milestone))
            .collect();
        subset.sort_by_key(|t| ticket_sort_key(t));
        lines.push(format!("## {milestone}"));
        lines.push("".into());
        for t in subset {
            let mark = if opt_str(t, "status") == Some("shipped") {
                "x"
            } else {
                " "
            };
            lines.push(format!(
                "- [{mark}] **{}** — {} (`{}`)",
                str_field(t, "id"),
                opt_str(t, "title").unwrap_or(""),
                opt_str(t, "status").unwrap_or(""),
            ));
        }
        lines.push("".into());
    }
    lines.join("\n")
}

fn generate_ticket_brainstorm_md(registry: &Value) -> String {
    let mut lines = vec![
        AUTO_HEADER.to_string(),
        "# Ticket Brainstorm".into(),
        "".into(),
        "`idea` + `deferred` only.".into(),
        "".into(),
    ];
    let mut by_program: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for t in tickets(registry) {
        if matches!(opt_str(t, "status"), Some("idea" | "deferred")) {
            let prog = opt_str(t, "program").unwrap_or("platform").to_string();
            by_program.entry(prog).or_default().push(t);
        }
    }
    for (program, mut list) in by_program {
        lines.push(format!("## {program}"));
        lines.push("".into());
        list.sort_by_key(|t| ticket_sort_key(t));
        for t in list {
            let surfaces = string_list(t, "surfaces").unwrap_or_default().join(", ");
            lines.push(format!(
                "- **{}** ({}) — {} [{}] — {}",
                str_field(t, "id"),
                opt_str(t, "status").unwrap_or(""),
                opt_str(t, "title").unwrap_or(""),
                surfaces,
                opt_str(t, "summary").unwrap_or(""),
            ));
        }
        lines.push("".into());
    }
    lines.join("\n")
}

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

fn inject_marker_block(path: &Path, start: &str, end: &str, inner: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;
    if !text.contains(start) || !text.contains(end) {
        bail!("Missing markers in {}: {} / {}", path.display(), start, end);
    }
    let (before, rest) = text.split_once(start).unwrap();
    let (_, after) = rest.split_once(end).unwrap();
    let inner_r = inner.trim_end();
    let new_text = format!("{before}{start}\n{inner_r}\n{end}{after}");
    fs::write(path, new_text)?;
    Ok(())
}

fn inject_status_block(root: &Path, registry: &Value) -> Result<()> {
    let all = tickets(registry);
    let mut shipped: Vec<&Value> = all
        .iter()
        .filter(|t| opt_str(t, "status") == Some("shipped"))
        .collect();
    shipped.sort_by(|a, b| order_or(b, 9999).cmp(&order_or(a, 9999)));
    let latest = shipped
        .first()
        .map(|t| str_field(t, "id"))
        .unwrap_or_else(|| "T-066".into());

    let mut lines = vec![format!("**Latest shipped:** **{latest}**"), "".into()];

    let slice_row = all.iter().find(|t| {
        t.get("active_slice")
            .map(|v| is_truthy(Some(v)))
            .unwrap_or(false)
    });
    if let Some(slice_row) = slice_row {
        let slice_id = opt_str(slice_row, "active_slice").unwrap_or("");
        let slice_read = slice_spec(slice_row);
        lines.push(format!(
            "**ACTIVE NOW:** **{}** — {slice_id} ({}). Slice spec: `{slice_read}`.",
            str_field(slice_row, "id"),
            opt_str(slice_row, "title").unwrap_or(""),
        ));
    } else {
        let mut ready: Vec<&Value> = all
            .iter()
            .filter(|t| opt_str(t, "status") == Some("ready"))
            .collect();
        ready.sort_by_key(|t| order_or(t, 9999));
        if let Some(r) = ready.first() {
            lines.push(format!(
                "**ACTIVE NOW:** **{}** — {}",
                str_field(r, "id"),
                opt_str(r, "title").unwrap_or(""),
            ));
        }
    }
    lines.push("".into());
    lines.push("**Next (by order):**".into());
    let mut queued: Vec<&Value> = all
        .iter()
        .filter(|t| {
            matches!(opt_str(t, "status"), Some("queued" | "ready")) && order_truthy(t)
        })
        .collect();
    queued.sort_by_key(|t| order_or(t, 9999));
    for t in queued.into_iter().take(10) {
        lines.push(format!(
            "- **{}** — {} (`{}`)",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or(""),
        ));
    }
    inject_marker_block(
        &root.join("CLAUDE.md"),
        STATUS_MARKER_START,
        STATUS_MARKER_END,
        &lines.join("\n"),
    )
}

fn inject_next_block(root: &Path, registry: &Value) -> Result<()> {
    let mut lines = vec![
        "### Recommended next work (auto-generated)".into(),
        "".into(),
    ];
    let mut open_t: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| {
            matches!(
                opt_str(t, "status"),
                Some("ready" | "queued" | "running" | "review")
            ) && order_truthy(t)
        })
        .collect();
    open_t.sort_by_key(|t| (order_or(t, 9999), str_field(t, "id")));
    for t in open_t.into_iter().take(10) {
        lines.push(format!(
            "- **{}** — {} ({})",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or(""),
        ));
    }
    inject_marker_block(
        &root.join("docs/specs/Mission_Creator_Architecture/ROADMAP.md"),
        NEXT_MARKER_START,
        NEXT_MARKER_END,
        &lines.join("\n"),
    )
}
