//! The readiness rules: the plan gate, work titles and the ready-tier body.
//!
//! **Role:** reds a ready-class work ticket without a `plan`, a work ticket without a title,
//! and a ready, running, review or shipped work ticket with an empty ready-tier body field.
//! **Position:** under `ticket check`; `ops::mark_ready` enforces the same gates at promotion.
//! **Signals & state:** none; reads the typed corpus.
//! **Invariants:** quarantined tickets (nonempty `migration_legacy`) are exempt from the body
//! rule; an unloadable corpus is a finding.

use super::*;

/// The plan ready-gate: no plan, no ready. Every READY-class (ready/running/review) WORK ticket must carry
/// `plan`; whether the named file exists is [`check_spec_and_plan_files_exist`], the
/// existence rule every ticket file shares, so a missing plan file is reported once.
/// Work-only on purpose: a program's `spec` is the shared program authority and
/// programs are never dispatched as slices — the plan is the per-ticket execution
/// document. `ops::mark_ready` enforces the same gate at promotion time (with the
/// id-derived default path); this corpus-wide rule additionally catches `set-status`
/// promotions and hand-edits. Fail-closed on an unloadable corpus.
pub(super) fn check_plan_ready_gate(root: &Path) -> Vec<String> {
    let corpus = match ticket_model::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        let ticket_model::Ticket::Work(w) = ticket else {
            continue;
        };
        if !matches!(
            w.status.name(),
            ticket_model::StatusName::Ready
                | ticket_model::StatusName::Running
                | ticket_model::StatusName::Review
        ) {
            continue;
        }
        if w.plan.as_deref().is_none_or(|p| p.trim().is_empty()) {
            errors.push(format!(
                "{id}: {} work ticket without a plan — ready-class requires plan ({}; \
                 `ticket mark-ready {id} <spec> [plan]` defaults it)",
                w.status.name().as_str(),
                ticket_model::repository::documentation::PLAN_TEMPLATE
            ));
        }
    }
    errors
}

/// The idea-tier title rule: EVERY work ticket carries a nonempty title, corpus-wide. Only
/// EMPTINESS is corpus-wide:
/// the two real-title arms (`!= id`, `<= 10 words`) are history debt behind
/// [`ticket_model::TITLE_DEBT_PIN`] — see [`check_debt_pins`] — and are enforced on
/// CHANGED tickets by the ops post-image gate, so check never reds past titles wholesale. Fail-closed on an unloadable corpus.
pub(super) fn check_work_title_nonempty(root: &Path) -> Vec<String> {
    let corpus = match ticket_model::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let ticket_model::Ticket::Work(w) = ticket
            && w.title.trim().is_empty()
        {
            errors.push(format!(
                "{}: title required on every work ticket (idea tier)",
                w.id
            ));
        }
    }
    errors
}

/// The ready-tier body rule: every ready, running, review or shipped WORK ticket
/// carries the six body fields nonempty ([`ticket_model::empty_ready_tier_fields`] —
/// context, requirement, current_state, approach, verify, acceptance), corpus-wide.
/// Quarantine exemption: a nonempty `migration_legacy` exempts the ticket (content
/// exists, unprocessed — the drain fills the fields). Work-only: a program
/// aggregates its children's bodies. Composes without double-reporting: on
/// ready-class tickets an empty `main_goal`, `spec` or `acceptance` is the PARSE
/// refusal ([`ticket_model::Status::live_ready`] — the load itself fails) and `validate_row`
/// covers the registry Value view, so the `acceptance` entry here fires only on
/// shipped tickets, which neither of those covers. Fail-closed on an unloadable
/// corpus.
pub(super) fn check_ready_tier_body(root: &Path) -> Vec<String> {
    let corpus = match ticket_model::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        let ticket_model::Ticket::Work(w) = ticket else {
            continue;
        };
        if !matches!(
            w.status.name(),
            ticket_model::StatusName::Ready
                | ticket_model::StatusName::Running
                | ticket_model::StatusName::Review
                | ticket_model::StatusName::Shipped
        ) || !w.migration_legacy.is_empty()
        {
            continue;
        }
        let missing = ticket_model::empty_ready_tier_fields(w);
        if !missing.is_empty() {
            errors.push(format!(
                "{}: {} work ticket with empty ready-tier body fields: {} — ready-tier requires them nonempty; fill from the spec/plan or demote",
                w.id,
                w.status.name().as_str(),
                missing.join(", ")
            ));
        }
    }
    errors
}
