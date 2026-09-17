//! Queue views.

use super::*;

pub(super) fn generate_ticket_dev_queue_md(registry: &Value) -> String {
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

pub(super) fn generate_ticket_mod_queue_md(registry: &Value) -> String {
    let mut rows: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| {
            matches!(
                opt_str(t, "status"),
                Some("ready" | "queued" | "running" | "review")
            ) && matches!(slice_executor(t).as_str(), "workbench" | "human")
                && string_list(t, "targets")
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

pub(super) fn generate_milestones_md(registry: &Value) -> String {
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

pub(super) fn generate_ticket_brainstorm_md(registry: &Value) -> String {
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
            let prog = opt_str(t, "kind")
                .or_else(|| opt_str(t, "program"))
                .unwrap_or("work")
                .to_string();
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
