//! The execution briefing for one ticket, assembled entirely from that ticket's own fields.

use super::*;

use crate::repository::{TICKETS_DIR, WORKTREES_DIR};

/// The statuses a ticket may carry, mirroring the `status` definition in the ticket schema.
pub(super) const VALID_TICKET_STATUSES: &[&str] = &[
    "idea",
    "queued",
    "ready",
    "running",
    "review",
    "shipped",
    "deferred",
    "cancelled",
];

/// Print everything an executor needs to start `id`: where to read, where to work, what the
/// ticket asks for, and what it will be accepted against.
///
/// Every line comes from the ticket itself. Guidance that is true of one ticket belongs in that
/// ticket's `spec`, `plan` and body fields, where the author maintains it and `ticket check`
/// validates it — never in this command, which would make it invisible to both.
pub fn cmd_brief(_root: &Path, registry: &Value, id: &str) -> Result<()> {
    let t = require_ticket(registry, id);
    let tid = str_field(t, "id");
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{tid}"));
    let active = opt_str(t, "active_slice").unwrap_or("").to_string();
    let spec = slice_spec(t);
    let shipped = shipped_slices(t);

    println!("{tid} · {}", opt_str(t, "title").unwrap_or(""));
    if !active.is_empty() {
        println!("SLICE: {active}");
    }
    println!("READ: {spec} (slice spec — only source of truth for this slice)");
    if let Some(hub) = opt_str(t, "spec")
        && hub != spec
    {
        println!("HUB: {hub} (program context only)");
    }
    if let Some(plan) = opt_str(t, "plan") {
        println!("PLAN: {plan}");
    }
    println!("BRANCH: {branch}");
    println!(
        "EXECUTION: Default ship on main. Parallel tickets use worktree {WORKTREES_DIR}/TBD-{tid} \
         @ {branch} (merge to main when done). Docs-only slices (cursor-docs) may commit on main. \
         See {TICKETS_DIR}/README.md."
    );
    println!("TARGETS: {}", slice_targets(t).join(", "));
    println!("DO NOT: edit documentation");
    if !shipped.is_empty() {
        println!("DO NOT REOPEN (shipped): {}", shipped.join(", "));
    }

    print_list(t, "owns", "OWNS");
    if let Some(goal) = opt_str(t, "main_goal") {
        println!("GOAL: {goal}");
    }
    for (field, label) in BODY_FIELDS {
        print_list(t, field, label);
    }
    print_list(t, "citations", "CITATIONS");
    print_list(t, "acceptance", "ACCEPTANCE");
    Ok(())
}

/// The body a work ticket carries, in the order an executor reads it: why the work exists, what
/// it must do, where the tree stands today, how to get there, and how to prove it.
const BODY_FIELDS: &[(&str, &str)] = &[
    ("context", "CONTEXT"),
    ("requirement", "REQUIREMENT"),
    ("current_state", "CURRENT STATE"),
    ("approach", "APPROACH"),
    ("verify", "VERIFY"),
];

fn print_list(t: &Value, field: &str, label: &str) {
    let Some(entries) = string_list(t, field) else {
        return;
    };
    if entries.is_empty() {
        return;
    }
    println!("{label}:");
    for entry in entries {
        println!("  - {entry}");
    }
}
