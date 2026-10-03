//! The read-only verbs.
//!
//! **Role:** `ticket show`, `next`, `prompt`, `list`, `plan-batch`, `sparse-paths`,
//! `gap-round-trip` and `scope-histogram`, and the shared unknown-ticket refusal.
//! **Position:** called by the xtask `ticket` command group with the registry value; the
//! histogram reads the typed corpus.
//! **Signals & state:** none; prints to stdout and never writes.
//! **Invariants:** an id the registry does not hold is refused with [`unknown_ticket`], which the
//! command prints verbatim.

use super::*;
use crate::error::{Error, Result, ResultExt};
use std::collections::BTreeMap;

/// The refusal for an id the registry does not hold; the command prints it verbatim and exits 1.
pub fn unknown_ticket(id: &TicketId) -> Error {
    Error::Refused {
        message: format!("Unknown ticket: {id}"),
    }
}

/// The registry row of `id`, or the [`unknown_ticket`] refusal.
pub fn require_ticket<'a>(registry: &'a Value, id: &TicketId) -> Result<&'a Value> {
    ticket_by_id(registry, id).ok_or_else(|| unknown_ticket(id))
}

/// Print ticket `id` as a Markdown card: id and title, program and surfaces, the route when
/// set, impact, status and order, then the remaining fields and the spec when set.
///
/// # Errors
/// The [`unknown_ticket`] refusal.
pub fn cmd_show(registry: &Value, id: &TicketId) -> Result<()> {
    let t = require_ticket(registry, id)?;
    let surfaces = string_list(t, "surfaces").unwrap_or_default().join(", ");
    let impact = string_list(t, "impact").unwrap_or_default().join(", ");
    println!(
        "### {} · {}",
        str_field(t, "id"),
        opt_str(t, "title").unwrap_or("")
    );
    println!(
        "**Program:** {} · **Where:** {surfaces}",
        opt_str(t, "program").unwrap_or("")
    );
    if let Some(route) = opt_str(t, "route") {
        println!("**Route:** {route}");
    }
    println!(
        "**Impact:** {impact} · **Status:** {} · **Order:** {}",
        opt_str(t, "status").unwrap_or(""),
        match t.get("order") {
            Some(v) if !matches!(v, Value::Null) => {
                if let Some(n) = v.as_i64() {
                    n.to_string()
                } else {
                    v.to_string()
                }
            }
            _ => "—".into(),
        }
    );
    println!("**Summary:** {}", opt_str(t, "summary").unwrap_or(""));
    if let Some(deps) = string_list(t, "depends_on")
        && !deps.is_empty()
    {
        println!("**Needs:** {}", deps.join(", "));
    }
    if let Some(unblocks) = string_list(t, "unblocks")
        && !unblocks.is_empty()
    {
        println!("**Blocks:** {}", unblocks.join(", "));
    }
    if let Some(spec) = opt_str(t, "spec") {
        println!("**Spec:** `{spec}`");
    }
    Ok(())
}

/// Print the active slice (the first row with a set `active_slice`), then the first five
/// `ready` or `queued` tickets that carry an order, in order.
///
/// # Errors
/// None in practice; the signature matches the other verbs.
pub fn cmd_next(registry: &Value) -> Result<()> {
    if let Some(slice_row) = tickets(registry).iter().find(|t| {
        t.get("active_slice")
            .map(|v| is_truthy(Some(v)))
            .unwrap_or(false)
    }) {
        println!(
            "ACTIVE: {} slice {}",
            str_field(slice_row, "id"),
            opt_str(slice_row, "active_slice").unwrap_or("")
        );
    }
    let mut open_t: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| matches!(opt_str(t, "status"), Some("ready" | "queued")) && order_truthy(t))
        .collect();
    open_t.sort_by_key(|t| ticket_sort_key(t));
    for t in open_t.into_iter().take(5) {
        println!(
            "  {} — {} ({})",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or("")
        );
    }
    Ok(())
}

/// Print the agent prompt of `id`'s spec (the given `slice`'s, else the active slice's, else the
/// ticket's own), preceded with `header` by the prompt's origin and handoff path.
///
/// # Errors
/// The [`unknown_ticket`] refusal, or [`Error::Refused`] for an unknown slice, a missing spec
/// or spec file, or a spec without a prompt section.
pub fn cmd_prompt(
    root: &Path,
    registry: &Value,
    id: &TicketId,
    slice: Option<&str>,
    header: bool,
) -> Result<()> {
    let t = require_ticket(registry, id)?;
    let slice_id = slice
        .map(|s| s.to_string())
        .or_else(|| opt_str(t, "active_slice").map(|s| s.to_string()));
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    let spec_rel = if let Some(s) = slice {
        let plan = plan.with_context(|| format!("Unknown slice {s} on {id}"))?;
        if !plan.contains_key(s) {
            return Err(Error::Refused {
                message: format!("Unknown slice {s} on {id}"),
            });
        }
        plan.get(s)
            .and_then(|r| r.get("spec"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        slice_spec(t)
    };
    if spec_rel.is_empty() {
        let sid = slice_id
            .as_deref()
            .map(|s| format!(" slice {s}"))
            .unwrap_or_default();
        return Err(Error::Refused {
            message: format!("No spec for {id}{sid}"),
        });
    }
    let spec_path = root.join(&spec_rel);
    if !spec_path.is_file() {
        return Err(Error::Refused {
            message: format!("Spec not found: {spec_rel}"),
        });
    }
    let text = fs::read_to_string(&spec_path)?;
    let prompt = match extract_prompt(&text) {
        Ok(p) => p,
        Err(e) => {
            return Err(Error::Refused {
                message: e.to_string(),
            });
        }
    };
    if header {
        let handoff = slice_handoff_path(t, slice_id.as_deref().map(TicketId::new).as_ref());
        let label = slice_id.unwrap_or_else(|| id.to_string());
        println!("# Prompt for {label} — from {spec_rel}");
        println!("# Handoff: {handoff}");
        println!();
    }
    println!("{prompt}");
    Ok(())
}

/// Print the `queue.json` run settings, then its tickets as a table of id, status, spec
/// (truncated to 50 characters) and title.
///
/// # Errors
/// When `queue.json` exists but cannot be read or parsed.
pub fn cmd_list(root: &Path, registry: &Value) -> Result<()> {
    let queue_path = root.join(repository_layout::QUEUE_JSON);
    let data = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let batch = data
        .get("batch_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(10);
    let conc = data
        .get("concurrency")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    println!("batch_size={batch} concurrency={conc}");
    println!("{:<8} {:<10} {:<50} TITLE", "ID", "STATUS", "SPEC");
    println!("{}", "-".repeat(100));
    if let Some(arr) = data.get("tickets").and_then(|t| t.as_array()) {
        for t in arr {
            let id = opt_str(t, "id").unwrap_or("");
            let status = opt_str(t, "status").unwrap_or("");
            let spec = opt_str(t, "spec").unwrap_or("");
            let spec_trunc: String = spec.chars().take(48).collect();
            let title = opt_str(t, "title").unwrap_or("");
            println!("{id:<8} {status:<10} {spec_trunc:<50} {title}");
        }
    }
    Ok(())
}

/// Print the next batch candidates: the first ten `queued` or `ready` tickets that carry an
/// order, with their spec or `(no spec yet)`.
///
/// # Errors
/// None in practice; the signature matches the other verbs.
pub fn cmd_plan_batch(registry: &Value) -> Result<()> {
    let mut queued: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| matches!(opt_str(t, "status"), Some("queued" | "ready")) && order_truthy(t))
        .collect();
    queued.sort_by_key(|t| ticket_sort_key(t));
    println!("Next batch candidates (top 10 by order):");
    for t in queued.into_iter().take(10) {
        let spec = opt_str(t, "spec").unwrap_or("(no spec yet)");
        println!(
            "  {} — {} [{}] — {spec}",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or("")
        );
    }
    Ok(())
}

/// The directories a sparse checkout needs to execute `id`: the workflow definitions every
/// target needs, plus one set per target the ticket names.
pub fn cmd_sparse_paths(registry: &Value, id: &TicketId) -> Result<()> {
    let t = require_ticket(registry, id)?;
    let mut paths = std::collections::BTreeSet::new();
    paths.insert(".github".to_string());
    for tgt in slice_targets(t) {
        let Some((_, set)) = ticket_model::repository::SPARSE_CHECKOUT_SETS
            .iter()
            .find(|(name, _)| *name == tgt)
        else {
            continue;
        };
        for p in *set {
            paths.insert((*p).to_string());
        }
    }
    for p in paths {
        println!("{p}");
    }
    Ok(())
}

/// Prove the gap-analysis tables round-trip and print `round-trip OK`.
///
/// # Errors
/// When the document cannot be read or does not round-trip.
pub fn cmd_gap_round_trip(root: &Path) -> Result<()> {
    test_gap_analysis_round_trip(root)?;
    println!("round-trip OK");
    Ok(())
}

/// `ticket scope-histogram` — read-only census of the typed corpus: per domain/layer/component
/// counts, per-surface counts, the "U surface-empty (scope ∈ estimated: E)" honesty counters per
/// component bucket, and the work-ticket class distribution.
pub fn cmd_scope_histogram(root: &Path) -> Result<()> {
    let corpus = Corpus::load(root).map_err(Error::msg)?;
    let mut works = 0usize;
    let mut programs = 0usize;
    let mut buckets: BTreeMap<String, Vec<&ticket_model::WorkTicket>> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for t in corpus.tickets.values() {
        match t {
            Ticket::Program(_) => programs += 1,
            Ticket::Work(w) => {
                works += 1;
                let key = match &w.scope.component {
                    Some(c) => format!("{}/{}/{c}", w.scope.domain.as_str(), w.scope.layer),
                    None => format!("{}/{}", w.scope.domain.as_str(), w.scope.layer),
                };
                buckets.entry(key).or_default().push(w);
                *classes
                    .entry(w.class.clone().unwrap_or_else(|| "(none)".into()))
                    .or_default() += 1;
            }
        }
    }
    println!("scope histogram — {works} work tickets, {programs} programs");
    for (key, tickets) in &buckets {
        println!("{key}: {}", tickets.len());
        let mut surfaces: BTreeMap<&str, usize> = BTreeMap::new();
        let mut empty = 0usize;
        let mut empty_marked = 0usize;
        for w in tickets {
            if w.scope.surface.is_empty() {
                empty += 1;
                if w.estimated.iter().any(|e| e == "scope") {
                    empty_marked += 1;
                }
            }
            for s in &w.scope.surface {
                *surfaces.entry(s.as_str()).or_default() += 1;
            }
        }
        if !surfaces.is_empty() {
            let list: Vec<String> = surfaces.iter().map(|(s, n)| format!("{s} {n}")).collect();
            println!("  surfaces: {}", list.join(", "));
        }
        if empty > 0 {
            println!("  {empty} surface-empty (scope ∈ estimated: {empty_marked})");
        }
    }
    let class_line: Vec<String> = classes.iter().map(|(c, n)| format!("{c} {n}")).collect();
    println!("class distribution (work): {}", class_line.join(", "));
    Ok(())
}
