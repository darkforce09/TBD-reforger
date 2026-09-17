//! Registry views.

use super::*;

pub(super) fn em_dash_or_order(t: &Value) -> String {
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

pub(super) fn generate_ticket_registry_md(registry: &Value) -> String {
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
            opt_str(t, "kind")
                .or_else(|| opt_str(t, "program"))
                .unwrap_or(""),
            opt_str(t, "title").unwrap_or(""),
            summary,
        ));
    }
    lines.push("".into());
    lines.join("\n")
}

pub(super) fn generate_ticket_lead_md(registry: &Value) -> String {
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
                if graph_ids.contains(&str_field(other, "id"))
                    && let Some(deps) = other.get("depends_on").and_then(|d| d.as_array())
                    && deps.iter().any(|d| d.as_str() == Some(tid.as_str()))
                {
                    graph_ids.insert(tid.clone());
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
                if let Some(dep_s) = dep.as_str()
                    && graph_ids.contains(dep_s)
                {
                    let dep_n = dep_s.replace('-', "");
                    let tid_n = tid.replace('-', "");
                    lines.push(format!("  {dep_n}[{dep_s}] --> {tid_n}[{tid}]"));
                }
            }
        }
    }
    lines.push("```".into());
    lines.push("".into());
    lines.join("\n")
}
