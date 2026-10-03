//! `ticket mark-ready`.
//!
//! **Role:** promotes one ticket to `ready` through the typed operation and syncs.
//! **Position:** called by the xtask `ticket` command group.
//! **Signals & state:** none; writes the ticket file and the sync outputs, and prints the result.
//! **Invariants:** the check preflight runs before the operation; the operation's refusals reach
//! the caller verbatim; promotion to `ready` does not change dispatchability, so no repack runs.

use super::*;

/// Promote `id` to `ready`, setting its spec from `spec_arg` and its plan from `plan_arg` (or
/// the default plan path), then sync and print `<id> -> ready (<spec>; plan <plan>)`.
///
/// # Errors
/// When the ticket is unknown, the check preflight is red, or a write fails; the operation's
/// refusals (spec, plan, dependencies, body) come back as [`Error::Refused`].
pub fn cmd_mark_ready(
    root: &Path,
    registry: &mut Value,
    id: &TicketId,
    spec_arg: Option<&str>,
    plan_arg: Option<&str>,
) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        return Err(unknown_ticket(id));
    }
    // Refuse ready promotion when the registry fails ticket check.
    require_check_ok(root, registry, &format!("mark-ready {id}"))?;

    // Typed op: spec-arg set, spec-on-disk + deps gates, ready promotion with the
    // exact main_goal (summary→title→id) and acceptance (["See spec."]) backfills. The
    // refusals — "Ticket {id} needs a spec path", "Spec file not found: …",
    // "Blocked by …" — come back verbatim, with the same exit code. The plan gate adds the
    // plan ready-gate: PLAN defaults to
    // documentation/tickets/plans/<id-lowercased-dots-to-underscores>_plan.md and must exist
    // on disk ("Plan file not found: …").
    let outcome = match ops::mark_ready(
        &mut corpus,
        id,
        spec_arg,
        plan_arg,
        &time_source::now_utc_rfc3339(),
    ) {
        Ok(o) => o,
        Err(msg) => return Err(refuse_verbatim(msg)),
    };
    let (spec, plan) = match corpus.get(id) {
        Some(Ticket::Work(w)) => (w.spec.clone(), w.plan.clone()),
        Some(Ticket::Program(p)) => (p.spec.clone(), p.plan.clone()),
        None => (None, None),
    };
    let spec = spec.unwrap_or_default().trim().to_string();
    let plan = plan.unwrap_or_default().trim().to_string();
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    // mark-ready syncs but does NOT repack: queued→ready is dispatchability-neutral.
    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} -> ready ({spec}; plan {plan})");
    Ok(())
}
